use std::fs;

use lib_core::math::vectors::custom::PrecisePositionC32;
use mte_macros::vpath_unchecked;

pub struct SaveManager {}

impl SaveManager {
    pub fn get_world_list(&self) -> Result<Vec<WorldHeader>, &str> {
        let mut worlds = Vec::new();
        let saves_path = vpath_unchecked!("data://saves/");
        if !saves_path.exists() {
            let _ = fs::create_dir_all(saves_path);
            return Ok(worlds);
        }

        if let Ok(entries) = fs::read_dir(saves_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    let world_file = path.join("world.data");
                    let world_data = match fs::read(world_file) {
                        Ok(data) => data,
                        Err(_) => continue,
                    };

                    let header = match bincode::deserialize::<WorldHeader>(&world_data) {
                        Ok(h) => h,
                        Err(_) => continue,
                    };

                    worlds.push(header);

                    // if let Some(world_name) = path.file_name().and_then(|n| n.to_str()) {
                    //     // worlds.push(world_name.to_string());
                    // }
                }
            }
        }

        Ok(worlds)
    }

    pub fn save_world_data(&self, header: WorldHeader) {
        let saves_path = vpath_unchecked!("data://saves/");
        if !saves_path.exists() {
            let _ = fs::create_dir_all(saves_path);
            return;
        }

        let overworld_path = saves_path.join("dimensions").join("overworld");

        if !overworld_path.exists() {
            let _ = fs::create_dir_all(overworld_path);
            return;
        }

        let file_path = saves_path.join(header.name.clone());

        let bytes = match bincode::serialize(&header) {
            Ok(b) => b,
            Err(_) => return,
        };

        fs::write(file_path, bytes).unwrap();
    }

    pub fn seed_to_rgb(seed: u32) -> [f32; 4] {
        // Оттенок от 0.0 до 360.0 градусов
        let hue = (seed % 360) as f32;
        let s = 0.85; // Насыщенность (85% — цвета яркие, но не выжигающие глаза)
        let v = 0.90; // Яркость (90%)

        // Классическая формула перевода HSV в RGB
        let c = v * s;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r_prime, g_prime, b_prime) = match (hue as u32) / 60 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        [r_prime + m, g_prime + m, b_prime + m, 1.0]
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WorldHeader {
    pub name: String,
    pub seed: u32,
    pub color: [f32; 4],
    pub player_dim: usize,
    pub player_pos: PrecisePositionC32,
}
