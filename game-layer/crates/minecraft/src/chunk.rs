use lib_renderer::renderer::block_greedy_renderer::types::Vertex;

use crate::coordinates::LocalCoords;
use crate::types::blocks::block::{Block, BlockType, PrerenderBlock};

pub type ChunkAssociatedType = u32;
pub const CHUNK_SIZE: u32 = 32;
pub const CHUNK_ARRAY_LEN: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

const _: () = assert!(CHUNK_SIZE == 32, "Код рассчитан на размер чанка 32x32x32");

pub struct Chunk {
    pub data: [BlockType; CHUNK_ARRAY_LEN],
    pub vram_slot_id: Option<usize>,
}

impl Chunk {
    pub fn from_block(block: BlockType) -> Self {
        Self {
            data: [block; CHUNK_ARRAY_LEN],
            vram_slot_id: None,
        }
    }

    pub fn set_block(&mut self, coords: LocalCoords, block: BlockType) {
        self.data[usize::from(coords)] = block
    }

    pub fn get_block(&self, coords: LocalCoords) -> BlockType {
        self.data[usize::from(coords)]
    }
}

pub fn into_new_array(data: &[BlockType; CHUNK_ARRAY_LEN]) -> [Block; CHUNK_ARRAY_LEN] {
    let mut result = [Block::default(); CHUNK_ARRAY_LEN];
    for (i, block_type) in data.iter().enumerate() {
        result[i] = Block::from_type(block_type.clone());
    }
    result
}

pub fn into_prerender_array(data: &[Block; CHUNK_ARRAY_LEN]) -> [PrerenderBlock; CHUNK_ARRAY_LEN] {
    let mut result = [PrerenderBlock::default(); CHUNK_ARRAY_LEN];
    for (i, block) in data.iter().enumerate() {
        result[i] = PrerenderBlock::from_block(block.clone());
    }
    result
}

pub fn generate_mesh(data: &[PrerenderBlock; CHUNK_ARRAY_LEN]) -> (Vec<Vertex>, Vec<u32>) {
    const C_SIZE: usize = CHUNK_SIZE as usize;
    let mut x_layers = [[0u32; C_SIZE]; C_SIZE];
    let mut y_layers = [[0u32; C_SIZE]; C_SIZE];
    let mut z_layers = [[0u32; C_SIZE]; C_SIZE];

    let mut current_index = 0;

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let mut y_mask = 0u32;
            let mut z_mask = 0u32;
            for x in 0..CHUNK_SIZE {
                let is_solid = data[current_index].get_type().is_solid();
                current_index += 1;

                if is_solid {
                    // Маска для осей, где строка — это направление X
                    // Устанавливаем x-ый бит в слое y, строке z
                    y_mask |= 1 << x;
                    // Маска для третьей оси (например, вид сбоку)
                    z_mask |= 1 << x;

                    // Маска для осей, где строка — это направление Z
                    // Устанавливаем z-ый бит в слое y, строке x
                    // (Обрати внимание, как меняются индексы, чтобы повернуть плоскость)
                    x_layers[x as usize][y as usize] |= 1 << z;
                }
            }
            y_layers[y as usize][z as usize] = y_mask;
            z_layers[z as usize][y as usize] = z_mask;
        }
    }

    let vertices: Vec<Vertex> = Vec::new();
    let indices: Vec<u32> = Vec::new();

    for y in 0..(CHUNK_SIZE - 1) {
        for z in 0..CHUNK_SIZE {
            let row_now = y_layers[y as usize][z as usize];
            let row_above = y_layers[(y + 1) as usize][z as usize];

            // Находим, где блок есть, а сверху пусто
            let mut visible_faces = row_now & !row_above;

            if visible_faces != 0 {
                while visible_faces != 0 {
                    // Находим индекс первой единицы (начало полигона по оси X)
                    let start_x = visible_faces.trailing_zeros();

                    // Находим длину цепочки единиц (ширину полигона по оси X)
                    let length = (visible_faces >> start_x).trailing_ones();

                    // КООРДИНАТЫ ГОТОВОГО ОТРЕЗКА:
                    // Начало: X = start_x, Y = y + 1, Z = z
                    // Длина по оси X = length

                    println!(
                        "Верхняя грань куба: X_start: {}, Y: {}, Z: {}, Ширина_X: {}",
                        start_x,
                        y + 1,
                        z,
                        length
                    );

                    // Стираем обработанный кусок из visible_faces
                    let mask_to_clear = ((1 << length) - 1) << start_x;
                    visible_faces &= !mask_to_clear;
                }
            };
        }
    }

    (vertices, indices)
}
