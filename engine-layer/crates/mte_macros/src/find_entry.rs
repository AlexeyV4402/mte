use std::path::PathBuf;

use lib_core::fs::os::get_crate_dir_from;
use lib_core::fs::vfs::types::PostBakeVfsEntry;
use proc_macro2::Span;

pub(crate) fn find_vfs_entry_full(
    target_path_str: &str,
    span: Span,
    from_file: Option<PathBuf>,
) -> Result<PostBakeVfsEntry, syn::Error> {
    let from_file = match from_file {
        Some(file) => file,
        None => {
            return Err(syn::Error::new(
                span,
                "VFS: Компилятор не смог определить файл вызова".to_string(),
            ));
        }
    };

    let crate_dir = get_crate_dir_from(&from_file)
        .map_err(|err| syn::Error::new(span, format!("Ошибка нахождения корня крейта: {}", err)))?;

    let meta_path = crate_dir.join(".vfs_meta");

    let data = std::fs::read(&meta_path).unwrap_or_else(|_| {
        panic!(
            "Не удалось прочитать файл метаданных по пути: {}. Запустите сборку заново.",
            meta_path.display()
        )
    });

    let meta_content: Vec<PostBakeVfsEntry> = bincode::deserialize(&data)
        .expect("Ошибка десериализации vfs.meta. Возможно, формат vfs изменился.");

    if let Some(entry) = meta_content
        .iter()
        .find(|entry| entry.vpath_str == target_path_str)
    {
        return Ok(entry.clone()); // Возвращаем объект целиком
    }

    let suggestions: Vec<String> = meta_content
        .iter()
        .map(|e| e.vpath_str.clone())
        .filter(|path| path.starts_with(target_path_str))
        .collect();

    if !suggestions.is_empty() {
        let error_message = format!(
            "Файл не найден. Возможные варианты:\n{}",
            suggestions.join("\n")
        );
        return Err(syn::Error::new(span, error_message));
    }

    Err(syn::Error::new(
        span,
        format!(
            "Виртуальный путь '{}' не существует в VFS!",
            target_path_str
        ),
    ))
}

pub(crate) fn find_vfs_entry(
    target_path_str: &str,
    span: Span,
    from_file: Option<PathBuf>,
) -> Result<(u64, u64, u64), syn::Error> {
    find_vfs_entry_full(target_path_str, span, from_file)
        .map(|entry| (entry.vpath_hash, entry.offset, entry.length))
}
