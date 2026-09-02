use glam::Vec2;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockVertex {
    pub packed_data: u32,
}

impl BlockVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<BlockVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Uint32,
            }],
        }
    }

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

impl GuiVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            // Шаг буфера равен размеру всей структуры GuiVertex в байтах
            array_stride: std::mem::size_of::<GuiVertex>() as wgpu::BufferAddress,
            // Данные меняются для каждой ВЕРШИНЫ (а не для инстанса)
            step_mode: wgpu::VertexStepMode::Vertex,
            // Описываем атрибуты, которые пойдут в шейдер под @location(0), (1) и (2)
            attributes: &[
                // @location(0) -> screen_pos: vec2<f32> (2 флоата = 8 байт)
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // @location(1) -> uv: vec2<f32> (2 флоата = 8 байт)
                // Смещаемся на размер первого поля (8 байт от начала структуры)
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // @location(2) -> color: vec4<f32> (4 флоата = 16 байт)
                // Смещаемся еще на 8 байт (итого 16 байт от начала структуры)
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct SimpleVertex {
    local_pos: [f32; 3],
}

impl SimpleVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SimpleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}
