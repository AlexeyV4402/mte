use glam::Vec2;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockVertex {
    pub packed_data: u32,
}

impl BlockVertex {
    #[inline(always)]
    pub fn new(x: u32, y: u32, z: u32, side: u32, block_type_id: u32) -> Self {
        Self {
            packed_data: (x & 0x3F)               // 0..5   (6 бит под X)
                | ((y & 0x3F) << 6)               // 6..11  (6 бит под Y)
                | ((z & 0x3F) << 12)              // 12..17 (6 бит под Z)
                | ((side & 0x7) << 18)            // 18..20 (3 бита под side_id)
                | (0u32 << 21)                    // 21..22 (2 бита под facing_id на будущее)
                | ((block_type_id & 0xFFF) << 23), // 23..34 (12 бит под 4096 типов блоков!)
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GuiVertex {
    pub screen_pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl GuiVertex {}

pub struct SimpleVertex {
    local_pos: [f32; 3],
}

impl SimpleVertex {}
