use noise::{Fbm, NoiseFn, SuperSimplex};

use crate::types::blocks::block::{Block, BlockType};
use crate::types::chunk::{CHUNK_ARRAY_LEN, Chunk};
use crate::types::coordinates::core::{ChunkCoords, LocalCoords};

pub trait WorldGenerator: Sync {
    fn new(seed: u32) -> Self;
    fn generate_chunk(&self, chunk_coords: ChunkCoords) -> Chunk;
}

pub struct SuperSimplexGenerator {
    noise: Fbm<SuperSimplex>,
    pub seed: u32,
}

impl WorldGenerator for SuperSimplexGenerator {
    fn new(seed: u32) -> Self {
        let mut noise = Fbm::<SuperSimplex>::new(seed);
        noise.octaves = 4;
        noise.persistence = 0.5;

        Self { noise, seed }
    }

    /// Генерирует массив блоков для конкретного чанка
    fn generate_chunk(&self, chunk_coords: ChunkCoords) -> Chunk {
        let mut array = [Block::air(); CHUNK_ARRAY_LEN];

        // Базовое смещение блоков этого чанка в мировом пространстве (в блоках)
        let world_chunk = chunk_coords.0.shl_all(5);
        let (world_chunk_x, world_chunk_y, world_chunk_z) = world_chunk.into();

        // Частота шума: чем меньше число, тем более растянутыми и гигантскими будут биомы/горы
        let frequency = 0.005;
        // Базовая высота ландшафта (например, уровень моря на Y = 64)
        let sea_level = 64;
        let max_height_variance = 64.0; // Максимальная высота гор от уровня моря

        for x in 0..32 {
            for z in 0..32 {
                // 1. Считаем абсолютные координаты блока в мире
                let abs_x = world_chunk_x + x as i32;
                let abs_z = world_chunk_z + z as i32;

                // 2. Сэмплируем 2D-шум высоты для этой колонки (X, Z)
                let noise_val = self
                    .noise
                    .get([abs_x as f64 * frequency, abs_z as f64 * frequency])
                    as f32; // Результат шума всегда от -1.0 до 1.0

                // 3. Вычисляем финальную высоту земли в этой точке мира
                let terrain_height = sea_level + (noise_val * max_height_variance) as i32;

                for y in 0..32 {
                    let abs_y = world_chunk_y + y as i32;

                    // 4. Заполняем куб блоками на основе высоты
                    let idx = usize::from(LocalCoords::new(x, y, z));
                    if abs_y < terrain_height - 3 {
                        array[idx] = Block::from_type(BlockType::Stone); // Глубоко под землей — камень
                    } else if abs_y < terrain_height {
                        array[idx] = Block::from_type(BlockType::Dirt); // Подпушка из земли
                    } else if abs_y == terrain_height {
                        array[idx] = Block::from_type(BlockType::Grass); // Верхний слой — трава
                    } else {
                        array[idx] = Block::from_type(BlockType::Air); // Всё что выше — воздух
                    }
                }
            }
        }

        Chunk {
            data: array,
            vram_slot_id: None,
            is_changed: true,
        }
    }
}
