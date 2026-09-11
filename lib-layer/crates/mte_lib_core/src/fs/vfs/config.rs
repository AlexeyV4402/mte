use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::fs::vfs::builder::BUILDER_STATE;

pub fn read_vfs_path_toml(crate_dir: &Path) -> Result<HashMap<String, PathConfig>, String> {
    let mut workspace_config: VfsConfig = BUILDER_STATE
        .get()
        .expect("Сборщик не инициализирован")
        .workspace_vfs_toml
        .clone();

    let crate_str = std::fs::read_to_string(crate_dir.join("vfs.toml")).unwrap_or_default();
    let crate_config: VfsConfig = toml::from_str(&crate_str).unwrap_or_default();

    workspace_config.merge_with(crate_config);

    workspace_config.normalize();

    if workspace_config.path.len() == 0 {
        return Err("".to_string());
    }

    Ok(workspace_config.path)
}

pub fn read_vfs_packs_toml(crate_dir: &Path) -> Result<Vec<VfsPack>, String> {
    let mut workspace_config: VfsConfig = BUILDER_STATE
        .get()
        .expect("Сборщик не инициализирован")
        .workspace_vfs_toml
        .clone();

    let crate_str = std::fs::read_to_string(crate_dir.join("vfs.toml")).unwrap_or_default();
    let crate_config: VfsConfig = toml::from_str(&crate_str).unwrap_or_default();

    workspace_config.merge_with(crate_config);

    workspace_config.normalize();

    if workspace_config.packs.len() == 0 {
        return Err("".to_string());
    }

    Ok(workspace_config.packs)
}

#[derive(Debug, Deserialize, Clone)]
pub struct PathConfig {
    pub path: String,
    #[serde(default)]
    pub path_type: PathType,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathType {
    #[default]
    #[serde(rename = "ospath")]
    OSPath,
    #[serde(rename = "vpath")]
    VPath,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct VfsConfig {
    #[serde(default)]
    pub path: HashMap<String, PathConfig>,
    #[serde(default)]
    pub packs: Vec<VfsPack>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VfsPack {
    pub output_name: String,
    pub vfile_name: String,
    pub desc: String,
    #[serde(default)]
    pub files: Vec<InPackFile>,
    #[serde(default)]
    pub dirs: Vec<InPackDir>,
}

#[derive(Debug, Deserialize, Clone, Default, Copy)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    Unpack,
    None,
    #[default]
    Auto,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InPackFile {
    pub path: PathBuf,
    pub path_type: PathType,
    pub in_pack_path: Option<PathBuf>,
    #[serde(default)]
    pub compression: CompressionType,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InPackDir {
    pub path: PathBuf,
    #[serde(default)]
    pub path_type: PathType,
    pub in_pack_path: Option<PathBuf>,
    #[serde(default)]
    pub compressions: HashMap<String, CompressionType>,
}

impl InPackDir {
    pub fn normalize(&mut self) {
        if self.in_pack_path.is_none() {
            self.in_pack_path = self.path.file_name().map(PathBuf::from);
        }
    }
}

impl InPackFile {
    pub fn normalize(&mut self) {
        if self.in_pack_path.is_none() {
            self.in_pack_path = self.path.file_name().map(PathBuf::from);
        }
    }
}

impl VfsConfig {
    pub fn normalize(&mut self) {
        self.packs.iter_mut().for_each(|pack| {
            pack.dirs.iter_mut().for_each(|dir| {
                dir.normalize();
            });
            pack.files.iter_mut().for_each(|file| file.normalize())
        });
    }

    pub fn merge_with(&mut self, higher_priorty: Self) {
        if !higher_priorty.path.is_empty() {
            self.path = higher_priorty.path;
        }
        if !higher_priorty.packs.is_empty() {
            self.packs = higher_priorty.packs;
        }
    }
}
