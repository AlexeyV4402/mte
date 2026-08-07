use std::path::Path;
use std::sync::OnceLock;

use crate::fs::vfs::config::VfsConfig;

pub struct BuilderState {
    pub workspace_vfs_toml: VfsConfig,
}

pub static BUILDER_STATE: OnceLock<BuilderState> = OnceLock::new();

#[inline]
pub fn init_builder_state(workspace_root: &Path) -> Result<(), String> {
    if BUILDER_STATE.get().is_some() {
        return Ok(());
    }

    let workspace_str =
        std::fs::read_to_string(workspace_root.join("vfs.toml")).unwrap_or_default();
    let workspace_config = toml::from_str::<VfsConfig>(&workspace_str).unwrap_or_default();

    BUILDER_STATE
        .set(BuilderState {
            workspace_vfs_toml: workspace_config,
        })
        .ok()
        .expect("Ошибка инициализации состояния сборщика.");

    Ok(())
}
