use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use lib_renderer::renderer::block_grid_renderer::types::BlockVertex;

use crate::types::blocks::block::{Block, BlockType};
use crate::types::coordinates::core::{InternalCoords, LocalCoords};
use crate::utils::mesher::{generate_mesh, into_prerender_array};

pub type ChunkAssociatedType = u32;
pub const CHUNK_SIZE: u32 = 32;
pub const CHUNK_SIZE_WITH_PADDING: u32 = CHUNK_SIZE + 2;
pub const CHUNK_ARRAY_LEN: usize =
    (CHUNK_SIZE_WITH_PADDING * CHUNK_SIZE_WITH_PADDING * CHUNK_SIZE_WITH_PADDING) as usize;

pub const VISIBLE_SIZE_RANGE: std::ops::RangeInclusive<u32> = 0..=(CHUNK_SIZE - 1);
pub const VIRT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 0..=(CHUNK_SIZE + 1);

const _: () = assert!(CHUNK_SIZE == 32, "Код рассчитан на размер чанка 32x32x32");

pub struct Chunk {
    pub data: [Block; CHUNK_ARRAY_LEN],
    pub vram_slot_id: Option<usize>,
}

impl Chunk {
    pub fn air() -> Self {
        Self {
            data: [Block::air(); CHUNK_ARRAY_LEN],
            vram_slot_id: None,
        }
    }

    pub fn from_block(block: Block) -> Self {
        let mut data = [Block::air(); CHUNK_ARRAY_LEN];
        for y in VISIBLE_SIZE_RANGE {
            for z in VISIBLE_SIZE_RANGE {
                for x in VISIBLE_SIZE_RANGE {
                    data[usize::from(LocalCoords::from((x, y, z)))] = block;
                }
            }
        }
        Self {
            data,
            vram_slot_id: None,
        }
    }

    pub fn from_block_as_grid(block: Block, grid_step: u32) -> Self {
        let mut data = [Block::air(); CHUNK_ARRAY_LEN];

        for y in VISIBLE_SIZE_RANGE {
            for z in VISIBLE_SIZE_RANGE {
                for x in VISIBLE_SIZE_RANGE {
                    if x % grid_step == 0 && y % grid_step == 0 && z % grid_step == 0 {
                        data[usize::from(LocalCoords::from((x, y, z)))] = block;
                    }
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

    pub fn set_block_raw(&mut self, internal_coords: InternalCoords, block: Block) {
        self.data[usize::from(internal_coords)] = block;
    }

    pub fn get_block(&self, coords: LocalCoords) -> Block {
        self.data[usize::from(coords)]
    }

    pub fn get_mesh(&self) -> BlockIndexedPrimitive {
        let prerender_arr = into_prerender_array(&self.data);
        generate_mesh(&prerender_arr)
    }
}
