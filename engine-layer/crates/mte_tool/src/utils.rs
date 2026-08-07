use std::path::{Path, PathBuf};

use infer::Infer;
use walkdir::WalkDir;

pub(crate) fn select_gpu_format(file_name: &str) -> (wgpu::TextureFormat, &str) {
    if file_name.ends_with("_normal") {
        (wgpu::TextureFormat::Bc5RgUnorm, "bc5")
    } else if file_name.ends_with("_orm") {
        (wgpu::TextureFormat::Bc7RgbaUnorm, "bc7")
    } else {
        // Texture + UI
        (wgpu::TextureFormat::Bc7RgbaUnormSrgb, "bc7")
    }
}

pub(crate) fn get_rs_files<P: AsRef<Path>>(root: P) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "rs"))
}

#[inline]
pub(crate) fn detect_file_format(bytes: &[u8]) -> Option<infer::Type> {
    let mut info = Infer::new();

    info.add("model/gltf-binary", "glb", |buf| {
        buf.len() >= 4 && &buf[0..4] == b"glTF"
    });

    info.get(bytes)
}
