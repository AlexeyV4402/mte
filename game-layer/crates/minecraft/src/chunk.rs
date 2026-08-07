use crate::blocks::Block;
use crate::coordinates::LocalCoords;

pub const CHUNK_SIZE: u32 = 32;
pub const CHUNK_ARRAY_LEN: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

const _: () = assert!(CHUNK_SIZE == 32, "Код рассчитан на размер чанка 32x32x32");

pub struct Chunk {
    pub data: [Block; CHUNK_ARRAY_LEN],
    pub vram_slot_id: Option<usize>,
}

impl Chunk {
    pub fn from_block(block: Block) -> Self {
        let mut data = [block; CHUNK_ARRAY_LEN];
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    // if x % 2 == 0 {
                    //     data[usize::from(LocalCoords::from((x, y, z)))] = Block::Air;
                    // }
                }
            }
        }
        Self {
            data,
            vram_slot_id: None,
        }
    }

    pub fn set_block(&mut self, coords: LocalCoords, block: Block) {
        self.data[usize::from(coords)] = block
    }

    pub fn get_block(&self, coords: LocalCoords) -> Block {
        self.data[usize::from(coords)]
    }
}
