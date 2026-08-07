use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn get_files_recursively<P: AsRef<Path>>(root: P) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
}

#[inline]
pub fn find_cargo(marker: &str, from: &Path) -> Result<PathBuf, String> {
    let mut current = from;
    loop {
        let manifest = current.join("Cargo.toml");
        if manifest.exists() {
            if let Ok(code) = fs::read_to_string(&manifest) {
                if code.contains(marker) {
                    return Ok(current.to_path_buf());
                }
            }
        }
        current = current
            .parent()
            .ok_or_else(|| format!("Вышли за пределы дерева в поисках Cargo.toml с {}", marker))?;
    }
}

pub const fn hash_vpath(path: &str) -> u64 {
    let bytes = path.as_bytes();
    let mut hash = 0xcbf29ce484222325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

#[inline]
pub fn get_workspace_dir() -> Result<PathBuf, String> {
    find_cargo("[workspace]", Path::new(env!("CARGO_MANIFEST_DIR")))
}

#[inline]
pub fn get_workspace_dir_from(from: &Path) -> Result<PathBuf, String> {
    find_cargo("[workspace]", from)
}

#[inline]
pub fn get_crate_dir_from(from: &Path) -> Result<PathBuf, String> {
    find_cargo("[package]", from)
}
