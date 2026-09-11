use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::fs::os::{find_cargo, get_crate_dir_from, get_files_recursively, get_workspace_dir};
use crate::fs::vfs::config::{
    CompressionType, PathType, VfsPack, read_vfs_packs_toml, read_vfs_path_toml
};

#[derive(Debug, Clone)]
pub enum VfsResolverError {
    WorkspaceNotFound,
    CrateNotFound,
    CallerFileNotFound,
    InvalidPathFormat(String),
    FileNotFound(PathBuf),
    VfsTomlMissingOrCorrupted(String),
    UnknownPrefix(String),
    CyclicDependency(String),
}

impl fmt::Display for VfsResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceNotFound => write!(
                f,
                "VFS: Не удалось найти корень воркспейса (Cargo.toml с секцией [workspace])"
            ),
            Self::CrateNotFound => write!(f, "VFS: Не удалось найти корень текущего крейта"),
            Self::CallerFileNotFound => write!(f, "VFS: Компилятор не смог определить файл вызова"),
            Self::InvalidPathFormat(p) => write!(
                f,
                "VFS: Неверный формат виртуального пути '{}'. Ожидалась схема (например, workspace://...)",
                p
            ),
            Self::FileNotFound(p) => write!(f, "VFS: Путь не существует на диске: {}", p.display()),
            Self::VfsTomlMissingOrCorrupted(msg) => {
                write!(f, "VFS: Файл vfs.toml отсутствует или поврежден: {}", msg)
            }
            Self::UnknownPrefix(p) => write!(f, "VFS: Используется неизвестный префикс: '{}'", p),
            Self::CyclicDependency(chain) => write!(
                f,
                "VFS Критическая ошибка: Обнаружена циклическая зависимость префиксов: [{}]",
                chain
            ),
        }
    }
}

impl std::error::Error for VfsResolverError {}

#[inline]
pub fn resolve_path_checked(
    vpath: &str,
    workspace_root: &Path,
    from_file: Option<PathBuf>,
) -> Result<PathBuf, VfsResolverError> {
    let path = resolve_path_unchecked(vpath, workspace_root, from_file)?;
    path.exists()
        .then_some(path.clone())
        .ok_or_else(|| VfsResolverError::FileNotFound(path))
}

#[inline]
pub fn resolve_path_unchecked(
    vpath: &str,
    workspace_root: &Path,
    from_file: Option<PathBuf>,
) -> Result<PathBuf, VfsResolverError> {
    let mut visited = Vec::new();
    resolve_path_internal(
        Cow::Borrowed(vpath),
        workspace_root,
        from_file,
        &mut visited,
    )
}

fn resolve_path_internal(
    vpath: Cow<'_, str>,
    workspace_root: &Path,
    from_file: Option<PathBuf>,
    visited: &mut Vec<String>,
) -> Result<PathBuf, VfsResolverError> {
    let Some((scheme, path)) = vpath.split_once("://") else {
        return Err(VfsResolverError::InvalidPathFormat(vpath.into_owned()));
    };

    match scheme {
        "workspace" => Ok(workspace_root.join(path)),

        "crate" | "caller" => {
            let file_rel = from_file.ok_or(VfsResolverError::CallerFileNotFound)?;
            let from_file_abs = workspace_root.join(file_rel);

            let base_dir = if scheme == "crate" {
                find_cargo("[package]", &from_file_abs)
                    .map_err(|_| VfsResolverError::CrateNotFound)?
            } else {
                let mut dir = from_file_abs;
                if dir.is_file() {
                    dir.pop();
                }
                dir
            };
            Ok(base_dir.join(path))
        }

        custom_scheme => {
            if visited.contains(&custom_scheme.to_string()) {
                visited.push(custom_scheme.to_string());
                return Err(VfsResolverError::CyclicDependency(visited.join(" -> ")));
            }

            let file_rel = from_file
                .clone()
                .ok_or(VfsResolverError::CallerFileNotFound)?;
            let from_file_abs = workspace_root.join(file_rel);

            let crate_root = get_crate_dir_from(&from_file_abs)
                .map_err(|err| VfsResolverError::VfsTomlMissingOrCorrupted(err.to_string()))?;

            let paths = read_vfs_path_toml(&crate_root)
                .map_err(|err| VfsResolverError::VfsTomlMissingOrCorrupted(err.to_string()))?;

            match paths.get(custom_scheme) {
                Some(path_config) => match (path_config.path_type, path_config.path.clone()) {
                    (PathType::VPath, v) => {
                        visited.push(custom_scheme.to_string());

                        let next_vpath = Cow::Owned(v.to_string());

                        let base =
                            resolve_path_internal(next_vpath, workspace_root, from_file, visited)?;

                        visited.pop();
                        Ok(base.join(path))
                    }
                    (PathType::OSPath, v) => Ok(PathBuf::from(v).join(path)),
                },
                _ => Err(VfsResolverError::UnknownPrefix(custom_scheme.to_string())),
            }
        }
    }
}

pub fn primary_in_pack_resolve(
    vpath: &str,
    manifest_dir: &Path,
    packs: &Vec<VfsPack>,
    from_file_abs: &Path,
) -> Result<(PathBuf, String, CompressionType), VfsResolverError> {
    // Разбили входной vpath на префикс, название пака и хвост пути к файлу
    let Some((scheme, pack_name, tail)) = vpath.split_once("://").and_then(|(s_scheme, s_body)| {
        s_body
            .split_once("/")
            .map(|(s_pack, s_tail)| (s_scheme, s_pack, s_tail))
    }) else {
        return Err(VfsResolverError::InvalidPathFormat(vpath.to_owned()));
    };

    let tail_path = Path::new(tail);

    match scheme {
        "packs" => {
            // Получаем содержимое конкретного пака
            let pack = packs
                .iter()
                .find(|v| v.vfile_name == pack_name)
                .ok_or_else(|| VfsResolverError::FileNotFound(PathBuf::from(vpath)))?;

            // 1. Стратегия А: Ищем точное совпадение среди одиночных файлов
            let file = pack.files.iter().find(|f| {
                f.in_pack_path
                    .as_ref()
                    .map(|in_pack| in_pack == tail_path)
                    .unwrap_or(false)
            });

            if let Some(f) = file {
                let path = match f.path_type {
                    PathType::OSPath => f
                        .path
                        .exists()
                        .then_some(f.path.clone())
                        .ok_or(VfsResolverError::FileNotFound(PathBuf::from(vpath)))?,
                    PathType::VPath => {
                        let vpath_str = f.path.to_str().ok_or_else(|| {
                            VfsResolverError::VfsTomlMissingOrCorrupted(
                                "Невалидный UTF-8 в пути".into(),
                            )
                        })?;
                        resolve_path_checked(
                            vpath_str,
                            manifest_dir,
                            Some(from_file_abs.to_path_buf()),
                        )?
                    }
                };
                return Ok((path, pack.output_name.clone(), f.compression.clone()));
            }

            // 2. Стратегия Б: Ищем внутри зарегистрированных директорий пака
            for dir in &pack.dirs {
                // Защита от отсутствующего значения (берём имя папки как дефолт, если normalize() не отработал)
                let in_pack_base = dir
                    .in_pack_path
                    .as_ref()
                    .cloned()
                    .or_else(|| dir.path.file_name().map(PathBuf::from))
                    .ok_or_else(|| {
                        VfsResolverError::VfsTomlMissingOrCorrupted(
                            "Не удалось определить имя папки".into(),
                        )
                    })?;

                let dir_os_path = match dir.path_type {
                    PathType::OSPath => dir.path.clone(),
                    PathType::VPath => {
                        let vpath_str = dir.path.to_str().ok_or_else(|| {
                            VfsResolverError::VfsTomlMissingOrCorrupted(
                                "Невалидный UTF-8 в пути папки".into(),
                            )
                        })?;
                        resolve_path_checked(
                            vpath_str,
                            manifest_dir,
                            Some(from_file_abs.to_path_buf()),
                        )?
                    }
                };

                // Рекурсивно ищем файл внутри физической папки
                let file_os_path = get_files_recursively(&dir_os_path).find(|f| {
                    if let Ok(relative_to_dir) = f.strip_prefix(&dir_os_path) {
                        // println!("ass: {}; {}", in_pack_base.join(relative_to_dir).display(), tail_path.display());
                        in_pack_base.join(relative_to_dir) == tail_path
                    } else {
                        false
                    }
                });

                if let Some(p) = file_os_path {
                    // Безопасное извлечение расширения файла без паник
                    let compression_type = p
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .and_then(|ext_str| dir.compressions.get(ext_str))
                        .cloned()
                        .unwrap_or(CompressionType::default());

                    return Ok((p, pack.output_name.clone(), compression_type));
                }
            }

            Err(VfsResolverError::FileNotFound(PathBuf::from(vpath)))
        }
        _ => Err(VfsResolverError::UnknownPrefix(scheme.to_string())),
    }
}

pub fn primary_pack_resolve(
    vpaths: &Vec<String>,
    workspace_root: &Path,
    from_file: Option<PathBuf>,
) -> Result<Vec<Result<(PathBuf, String, CompressionType), VfsResolverError>>, VfsResolverError> {
    let file_rel = from_file.ok_or(VfsResolverError::CallerFileNotFound)?;
    let from_file_abs = workspace_root.join(file_rel);

    // Добываем список паков
    let packs = get_crate_dir_from(&from_file_abs)
        .and_then(|crate_root| read_vfs_packs_toml(&crate_root))
        .map_err(|err| VfsResolverError::VfsTomlMissingOrCorrupted(err.to_string()))?;

    Ok(vpaths
        .iter()
        .map(|vpath| primary_in_pack_resolve(vpath, workspace_root, &packs, &from_file_abs))
        .collect::<Vec<Result<(PathBuf, String, CompressionType), VfsResolverError>>>())
}

pub fn get_cache_path(src: &Path) -> Result<PathBuf, String> {
    let cache_name = src.to_string_lossy().replace("/", "|");
    Ok(get_workspace_dir()?
        .join(".mte/assets_cache")
        .join(cache_name))
}
