use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use lib_core::fs::os::{get_crate_dir_from, get_files_recursively};
use lib_core::fs::vfs::builder::init_builder_state;
use lib_core::fs::vfs::config::read_vfs_path_toml;
use lib_core::fs::vfs::resolve::resolve_path_checked;
use mime_guess::from_ext;
use mte_macros::vpath;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AssetCategory {
    Image,
    Audio,
    Shader,
    Unknown,
}

pub fn prepare(args: &[String], release: bool) -> Result<(), String> {
    let mut file_option: Option<PathBuf> = None;

    // println!("{:#?}", args);

    for argument in args.iter() {
        if argument == "--release" {
        } else if let Some(val) = argument.strip_prefix("--file=") {
            if !val.is_empty() {
                file_option = Some(PathBuf::from(val));
            }
        }
    }

    let file_path = match file_option.clone() {
        Some(p) => p,
        None => return Err("Необходимо указать путь к файлу, относительно которого будет найден корень крейта.
Это необходимо для нахождения папки ассетов. Убедитесь, что папка ассетов для крейта указана в vfs.toml".to_string()),
    };

    let workspace_root = vpath!("workspace://");

    init_builder_state(&workspace_root)
        .map_err(|err| format!("Ошибка инициализации состояния сборщика: {}", err))?;

    let crate_dir = get_crate_dir_from(&file_path)?;

    let mut vfs_paths = read_vfs_path_toml(&crate_dir)?;

    let assets_path = match vfs_paths.remove("assets") {
        Some(assets_path) => assets_path,
        None => return Err("В vfs.toml не указана папка assets".to_string()),
    };

    let assets_os_path = match assets_path.path_type {
        lib_core::fs::vfs::config::PathType::OSPath => PathBuf::from(assets_path.path),
        lib_core::fs::vfs::config::PathType::VPath => {
            resolve_path_checked(&assets_path.path, &workspace_root, file_option)
                .map_err(|err| format!("Ошибка разрешения путей: {}", err))?
        }
    };

    get_files_recursively(assets_os_path).for_each(|file| {
        let file_name = match file.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => {
                println!("Ошибка извлечения имени файла: {}", file.display());
                return;
            }
        };

        // 2. РАЗБИВАЕМ ИМЯ ПО ТОЧКАМ (Анализ с конца, без лишней аллокации в куче)
        let parts: Vec<&str> = file_name.split('.').collect();

        // Защита: если точек больше двух (например, "dirt.albedo.png"), проверяем предпоследнюю часть
        if parts.len() >= 3 {
            let current_purpose = parts[parts.len() - 2].to_lowercase();
            // Если там уже записан валидный purpose — пропускаем
            if VALID_PURPOSES.contains(&current_purpose.as_str()) {
                return;
            }
        }

        // 3. ЕСЛИ ФАЙЛ СЫРОЙ — ОПРЕДЕЛЯЕМ КАТЕГОРИЮ КЛАССИЧЕСКИМ ПУТЕМ
        let raw_ext = match file.extension().and_then(|e| e.to_str()) {
            Some(ext) => ext.to_lowercase(),
            None => return,
        };

        let mime = from_ext(&raw_ext).first();

        let mut asset_category = if let Some(m) = mime {
            match m.type_().as_str() {
                "image" => AssetCategory::Image,
                "audio" => AssetCategory::Audio,
                _ => AssetCategory::Unknown,
            }
        } else {
            AssetCategory::Unknown
        };

        // Перехватываем твои шейдеры до того, как они упадут в Unknown
        let mut final_ext = raw_ext.clone();
        if matches!(raw_ext.as_str(), "vert" | "frag" | "comp") {
            asset_category = AssetCategory::Shader;
            final_ext = "glsl".to_string();
        }

        if asset_category == AssetCategory::Unknown {
            return; // Мусор не трогаем
        }

        let purpose = get_default_purpose(asset_category, &raw_ext);
        let stem = file.file_stem().unwrap().to_string_lossy();

        // 4. ФИЗИЧЕСКОЕ ПЕРЕИМЕНОВАНИЕ
        match asset_category {
            AssetCategory::Image | AssetCategory::Shader => {
                // Формируем новое идеальное имя, например: "chunks.vertex.glsl"
                let new_name = format!("{}.{}.{}", stem, purpose, final_ext);
                let new_path = file.with_file_name(new_name);

                if let Err(err) = fs::rename(&file, &new_path) {
                    println!("Пиздец при переименовании файла {:?}: {}", file, err);
                }
            }
            AssetCategory::Audio => todo!("Сделать обработку аудио позже"),
            AssetCategory::Unknown => {}
        }
    });

    Ok(())
}

const VALID_PURPOSES: &[&str] = &[
    "albedo",
    "normal",
    "normal_map",
    "vertex",
    "fragment",
    "compute",
];

fn get_default_purpose(category: AssetCategory, raw_ext: &str) -> &'static str {
    match category {
        AssetCategory::Image => {
            if raw_ext.contains("normal") || raw_ext.contains("_n") {
                "normal"
            } else {
                "albedo"
            }
        }
        AssetCategory::Shader => match raw_ext {
            "vert" => "vertex",
            "frag" => "fragment",
            "comp" => "compute",
            _ => "unknown_shader",
        },
        _ => "default",
    }
}
