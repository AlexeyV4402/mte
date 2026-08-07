use std::path::PathBuf;

#[repr(C)]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PostBakeVfsEntry {
    pub vpath_hash: u64,
    pub vpath_str: String,
    pub pack_path: PathBuf,
    pub offset: u64,
    pub length: u64,
}
