use wgpu::wgt::DrawIndexedIndirectArgs;

use super::types::Vertex;

pub const MAX_BUFFER_SIZE: u64 = 268435456 * 4;

pub const GLOBAL_BUFFER_SECTION_COUNT: usize = 343;

pub const GLOBAL_INDIRECT_BUFFER_CAPACITY: usize =
    size_of::<DrawIndexedIndirectArgs>() * GLOBAL_BUFFER_SECTION_COUNT;

pub const GLOBAL_BUFFER_VERTEX_PER_SECTION: u32 = 196608;
pub const GLOBAL_BUFFER_INDEX_PER_SECTION: u32 =
    (GLOBAL_BUFFER_VERTEX_PER_SECTION as f32 * 1.5) as u32;

pub const GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY: usize =
    (size_of::<Vertex>() * GLOBAL_BUFFER_VERTEX_PER_SECTION as usize) as usize;
pub const GLOBAL_VERTEX_BUFFER_CAPACITY: usize =
    GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY * GLOBAL_BUFFER_SECTION_COUNT;

const _: () = assert!(
    GLOBAL_VERTEX_BUFFER_CAPACITY <= MAX_BUFFER_SIZE as usize,
    "Запрещено создание буферов больше 256 МБ"
);

pub const GLOBAL_INDEX_BUFFER_SECTION_CAPACITY: usize =
    (size_of::<u32>() * GLOBAL_BUFFER_INDEX_PER_SECTION as usize) as usize;
pub const GLOBAL_INDEX_BUFFER_CAPACITY: usize =
    GLOBAL_INDEX_BUFFER_SECTION_CAPACITY * GLOBAL_BUFFER_SECTION_COUNT;

const _: () = assert!(
    GLOBAL_INDEX_BUFFER_CAPACITY <= MAX_BUFFER_SIZE as usize,
    "Запрещено создание буферов больше 256 МБ"
);

pub const GLOBAL_MATRIX_BUFFER_CAPACITY: usize =
    GLOBAL_BUFFER_SECTION_COUNT as usize * std::mem::size_of::<[[f32; 4]; 4]>();

const _: () = assert!(
    GLOBAL_MATRIX_BUFFER_CAPACITY <= MAX_BUFFER_SIZE as usize,
    "Запрещено создание буферов больше 256 МБ"
);
