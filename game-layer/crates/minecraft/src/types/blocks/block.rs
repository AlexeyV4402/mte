use mte_macros::define_blocks;

use crate::types::blocks::properties::{Facing, Shape, TextureMappingProfile};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockProperty {
    pub base_id: u32,
    pub profile_id: u32,
}

define_blocks! {
    Air   => { solid: false, shape: Shape::None, profile: AllSides,       textures: ["packs://pack0/void.png"] },
    Dirt  => { solid: true,  shape: Shape::Full, profile: AllSides,       textures: ["packs://pack0/dirt.png"] },
    Ilya  => { solid: true,  shape: Shape::Full, profile: AllSides,       textures: ["packs://pack0/ilya.jpg"] },
    Grass => { solid: true,  shape: Shape::Full, profile: TopBottomSides, textures: ["packs://pack0/grass_block_top.png", "packs://pack0/dirt.png", "packs://pack0/grass_block_side.png"] },
    Stone => { solid: true,  shape: Shape::Full, profile: AllSides,       textures: ["packs://pack0/stone.png"] },
    OakLog => { solid: true,  shape: Shape::Full, profile: AxisAligned,   textures: ["packs://pack0/oak_log_top.png", "packs://pack0/oak_log_side.png"] }
}

#[derive(Default, Clone, Copy)]
pub struct PrerenderBlock(u32);

impl PrerenderBlock {
    pub fn as_u32(self) -> u32 {
        unsafe { std::mem::transmute::<Self, u32>(self) }
    }

    pub fn get_type(self) -> BlockType {
        let block_id = self.0 >> 20;
        unsafe { std::mem::transmute(block_id as u16) }
    }

    pub fn from_block(block: Block) -> Self {
        let block_type = block.get_type();
        let texture_mapping_profile = block_type.get_texture_mapping_profile();
        let shape = block_type.get_shape();

        let mut result = (block.as_u16() as u32) << 16;

        result |= unsafe {
            std::mem::transmute::<TextureMappingProfile, u8>(texture_mapping_profile) as u32
        } << 14;
        result |= unsafe { std::mem::transmute::<Shape, u8>(shape) as u32 } << 12;

        Self(result)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Block(u16);

impl Block {
    pub fn as_u16(self) -> u16 {
        unsafe { std::mem::transmute::<Self, u16>(self) }
    }

    pub fn from_type(block_type: BlockType) -> Self {
        Self(unsafe { std::mem::transmute::<BlockType, u16>(block_type) << 4 })
    }

    pub fn get_type(self) -> BlockType {
        let block_id = self.0 >> 4;
        unsafe { std::mem::transmute(block_id) }
    }

    pub fn air() -> Self {
        Self::from_type(BlockType::Air)
    }

    pub fn get_facing(self) -> Facing {
        let facing_id = ((self.0 >> 1) & 0x0007) as u8;
        unsafe { std::mem::transmute(facing_id) }
    }

    pub fn with_type(self, block_type: BlockType) -> Self {
        Self(unsafe { std::mem::transmute::<BlockType, u16>(block_type) << 4 | (self.0 & 0x000F) })
    }

    pub fn with_facing(self, facing: Facing) -> Self {
        Self(unsafe { (std::mem::transmute::<Facing, u8>(facing) as u16) << 1 | (self.0 & 0xFFF1) })
    }
}
