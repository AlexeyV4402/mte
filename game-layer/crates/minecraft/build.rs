use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use image::GenericImageView;
use mte_macros::{vpath, vpath_unchecked};

pub fn create_texture_array_buffer(paths: &Vec<PathBuf>, tile_size: u32) -> (Vec<u8>, u32) {
    let mut combined_bytes = Vec::new();
    let layer_count = paths.len() as u32;

    for path in paths {
        let img = image::open(path).expect("Не удалось загрузить текстуру блока");
        let (width, height) = img.dimensions();

        assert_eq!(
            width,
            tile_size,
            "Ширина текстуры {} не совпадает с заданной!",
            path.display()
        );
        assert_eq!(
            height,
            tile_size,
            "Высота текстуры {} не совпадает с заданной!",
            path.display()
        );

        // Переводим в RGBA8888 байты (по 4 байта на пиксель)
        let rgba_bytes = img.to_rgba8().into_raw();
        combined_bytes.extend(rgba_bytes);
    }

    (combined_bytes, layer_count)
}

pub fn main() -> Result<(), std::io::Error> {
    let asset_count = 6;

    if fs::metadata(vpath_unchecked!("content://blocks.pck"))
        .and_then(|m| Ok(m.len()))
        .unwrap_or(0)
        == 1024 * asset_count
    {
        return Ok(());
    }

    let paths: Vec<PathBuf> = vec![
        vpath!("workspace://game-layer/assets/minecraft/textures/blocks/void.png"),
        vpath!("workspace://game-layer/assets/minecraft/textures/blocks/dirt.png"),
        vpath!("workspace://game-layer/assets/minecraft/textures/blocks/ilya.jpg"),
        vpath!("workspace://game-layer/assets/minecraft/textures/blocks/grass_block_side.png"),
        vpath!("workspace://game-layer/assets/minecraft/textures/blocks/grass_block_top.png"),
        vpath!("workspace://game-layer/assets/minecraft/textures/blocks/stone.png"),
    ];

    assert_eq!(asset_count, paths.len() as u64);

    let bytes = create_texture_array_buffer(&paths, 16);
    let mut file = OpenOptions::new()
        .append(false)
        .write(true)
        .create(true)
        .open(vpath_unchecked!("content://blocks.pck"))
        .unwrap();
    file.write(&bytes.0);
    Ok(())
}
