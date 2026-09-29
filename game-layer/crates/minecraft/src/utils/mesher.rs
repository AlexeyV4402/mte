use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use lib_renderer::renderer::block_grid_renderer::types::BlockVertex;

use crate::types::blocks::block::{Block, PrerenderBlock};
use crate::types::chunk::{CHUNK_ARRAY_LEN, VIRT_SIZE_RANGE};

pub const SIDE_TOP: u32 = 0;
pub const SIDE_BOTTOM: u32 = 1;
pub const SIDE_NORTH: u32 = 2;
/// -Z
pub const SIDE_SOUTH: u32 = 3;
/// +Z
pub const SIDE_WEST: u32 = 4;
/// -X
pub const SIDE_EAST: u32 = 5;
/// +X

pub fn into_prerender_array<const N: usize>(data: &[Block; N]) -> [PrerenderBlock; N] {
    let mut result = [PrerenderBlock::default(); N];
    for (i, block) in data.iter().enumerate() {
        result[i] = PrerenderBlock::from_block(block.clone());
    }
    result
}

const INTERNAL_MASK: u64 = 0x1_FFFF_FFFE;

pub fn generate_mesh_34(data: &[PrerenderBlock; CHUNK_ARRAY_LEN]) -> BlockIndexedPrimitive {
    let mut vertices: Vec<BlockVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut present_materials = [false; 4096];
    let mut unique_materials = Vec::with_capacity(16);

    for block in data.iter() {
        let mat_id = block.get_type() as usize; // твои 12 бит
        if mat_id != 0 && !present_materials[mat_id] {
            present_materials[mat_id] = true;
            unique_materials.push(mat_id as u16);
        }
    }

    let mut vertex_counter: u32 = 0;

    for &current_mat in &unique_materials {
        // 1. Обнуляем y_layers для конкретного материала
        let mut y_layers = [[0u64; 34]; 34];
        let mut x_layers = [[0u64; 34]; 34];
        let mut z_layers = [[0u64; 34]; 34];

        let mut idx = 0;
        for y in VIRT_SIZE_RANGE {
            for z in VIRT_SIZE_RANGE {
                for x in VIRT_SIZE_RANGE {
                    let block = data[idx];
                    idx += 1;

                    if block.get_type().is_solid() && (block.get_type() as u16) == current_mat {
                        // Выставляем биты в u64. Координаты x, y, z гарантированно в пределах 0..33
                        y_layers[y as usize][z as usize] |= 1 << x;
                        x_layers[x as usize][y as usize] |= 1 << z;
                        z_layers[z as usize][y as usize] |= 1 << x;
                    }
                }
            }
        }

        let current_mat_u32 = current_mat as u32;

        // ==========================================
        // 1. ГЕНЕРАЦИЯ ГОРИЗОНТАЛЬНЫХ ГРАНЕЙ (TOP / BOTTOM)
        // ==========================================
        for y in 0..33 {
            for z in 1..33 {
                // Внутренние строки игрового чанка (индексы массива 1..32)
                let row_now = y_layers[y as usize][z as usize];
                let row_above = y_layers[(y + 1) as usize][z as usize];

                // println!("{:x}", row_now);

                let visible_faces_top = row_now & !row_above & INTERNAL_MASK;
                let visible_faces_bottom = row_above & !row_now & INTERNAL_MASK;

                // ВЕРХ (+Y) - строим только если это верхняя грань ИГРОВОГО блока (y >= 1)
                if y >= 1 && visible_faces_top != 0 {
                    mesh_bitline_34(
                        visible_faces_top,
                        z,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, z_val| {
                            // Переводим в isize для безопасного вычитания без паники
                            let gpu_y = y as u32;
                            let gpu_z = (z_val as isize - 1) as u32;
                            let x0 = (start_x as isize - 1) as u32;
                            let x1 = (start_x as isize - 1 + len_x as isize) as u32;
                            [
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_TOP, current_mat_u32),
                                BlockVertex::new(x1, gpu_y, gpu_z + 1, SIDE_TOP, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z + 1, SIDE_TOP, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_TOP, current_mat_u32),
                            ]
                        },
                    );
                }

                // НИЗ (-Y)
                if y < 32 && visible_faces_bottom != 0 {
                    mesh_bitline_34(
                        visible_faces_bottom,
                        z,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, z_val| {
                            let gpu_y = y as u32;
                            let gpu_z = (z_val as isize - 1) as u32;
                            let x0 = (start_x as isize - 1) as u32;
                            let x1 = (start_x as isize - 1 + len_x as isize) as u32;
                            [
                                BlockVertex::new(
                                    x1,
                                    gpu_y,
                                    gpu_z + 1,
                                    SIDE_BOTTOM,
                                    current_mat_u32,
                                ),
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_BOTTOM, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_BOTTOM, current_mat_u32),
                                BlockVertex::new(
                                    x0,
                                    gpu_y,
                                    gpu_z + 1,
                                    SIDE_BOTTOM,
                                    current_mat_u32,
                                ),
                            ]
                        },
                    );
                }
            }
        }

        // ==========================================
        // 2. ГЕНЕРАЦИЯ ВЕРТИКАЛЬНЫХ ГРАНЕЙ ПО X (EAST / WEST)
        // ==========================================
        for x in 0..33 {
            for y in 1..33 {
                let row_now = x_layers[x as usize][y as usize];
                let row_next = x_layers[(x + 1) as usize][y as usize];

                let visible_faces_east = row_now & !row_next & INTERNAL_MASK;
                let visible_faces_west = row_next & !row_now & INTERNAL_MASK;

                // ВОСТОК (+X)
                if x >= 1 && visible_faces_east != 0 {
                    mesh_bitline_34(
                        visible_faces_east,
                        y,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_z, len_z, y_val| {
                            let gpu_x = x as u32;
                            let gpu_y = (y_val as isize - 1) as u32;
                            let z0 = (start_z as isize - 1) as u32;
                            let z1 = (start_z as isize - 1 + len_z as isize) as u32;
                            [
                                BlockVertex::new(gpu_x, gpu_y, z1, SIDE_EAST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z1, SIDE_EAST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z0, SIDE_EAST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y, z0, SIDE_EAST, current_mat_u32),
                            ]
                        },
                    );
                }

                // ЗАПАД (-X)
                if x < 32 && visible_faces_west != 0 {
                    mesh_bitline_34(
                        visible_faces_west,
                        y,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_z, len_z, y_val| {
                            let gpu_x = x as u32;
                            let gpu_y = (y_val as isize - 1) as u32;
                            let z0 = (start_z as isize - 1) as u32;
                            let z1 = (start_z as isize - 1 + len_z as isize) as u32;
                            [
                                BlockVertex::new(gpu_x, gpu_y, z0, SIDE_WEST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z0, SIDE_WEST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z1, SIDE_WEST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y, z1, SIDE_WEST, current_mat_u32),
                            ]
                        },
                    );
                }
            }
        }

        // ==========================================
        // 3. ГЕНЕРАЦИЯ ВЕРТИКАЛЬНЫХ ГРАНЕЙ ПО Z (SOUTH / NORTH)
        // ==========================================
        for z in 0..33 {
            for y in 1..33 {
                let row_now = z_layers[z as usize][y as usize];
                let row_next = z_layers[(z + 1) as usize][y as usize];

                let visible_faces_south = row_now & !row_next & INTERNAL_MASK;
                let visible_faces_north = row_next & !row_now & INTERNAL_MASK;

                // ЮГ (+Z)
                if z >= 1 && visible_faces_south != 0 {
                    mesh_bitline_34(
                        visible_faces_south,
                        y,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, y_val| {
                            let gpu_z = z as u32;
                            let gpu_y = (y_val as isize - 1) as u32;
                            let x0 = (start_x as isize - 1) as u32;
                            let x1 = (start_x as isize - 1 + len_x as isize) as u32;
                            [
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_SOUTH, current_mat_u32),
                                BlockVertex::new(x0, gpu_y + 1, gpu_z, SIDE_SOUTH, current_mat_u32),
                                BlockVertex::new(x1, gpu_y + 1, gpu_z, SIDE_SOUTH, current_mat_u32),
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_SOUTH, current_mat_u32),
                            ]
                        },
                    );
                }

                // СЕВЕР (-Z)
                if z < 32 && visible_faces_north != 0 {
                    mesh_bitline_34(
                        visible_faces_north,
                        y,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, y_val| {
                            let gpu_z = z as u32;
                            let gpu_y = (y_val as isize - 1) as u32;
                            let x0 = (start_x as isize - 1) as u32;
                            let x1 = (start_x as isize - 1 + len_x as isize) as u32;
                            [
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_NORTH, current_mat_u32),
                                BlockVertex::new(x1, gpu_y + 1, gpu_z, SIDE_NORTH, current_mat_u32),
                                BlockVertex::new(x0, gpu_y + 1, gpu_z, SIDE_NORTH, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_NORTH, current_mat_u32),
                            ]
                        },
                    );
                }
            }
        }
    }

    BlockIndexedPrimitive::new(vertices, indices)
}

fn mesh_bitline_34<F>(
    mut visible_faces: u64, // Изменили тип на u64
    cross_coord: u32,
    vertex_counter: &mut u32,
    vertices: &mut Vec<BlockVertex>,
    indices: &mut Vec<u32>,
    mut build_quad: F,
) where
    F: FnMut(u32, u32, u32) -> [BlockVertex; 4],
{
    if visible_faces == 0 {
        return;
    }

    while visible_faces != 0 {
        let start = visible_faces.trailing_zeros();
        let length = (visible_faces >> start).trailing_ones();

        let packed_vertices = build_quad(start, length, cross_coord);

        vertices.extend_from_slice(&packed_vertices);

        let vc = *vertex_counter;
        indices.extend_from_slice(&[vc + 0, vc + 1, vc + 2, vc + 0, vc + 2, vc + 3]);

        *vertex_counter += 4;

        // Корректная очистка маски для u64
        let mask_to_clear = ((1u64 << length) - 1u64) << start;
        visible_faces &= !mask_to_clear;
    }
}

pub trait ChunkConfig {
    const REAL_SIZE: usize; // Чистый размер (32 или 8)
    const VIRT_SIZE: usize; // Размер массива в памяти (34 или 8)
    const HAS_PADDING: bool; // Есть ли паддинг

    // Индекс, с которого начинаются реальные блоки (1 для 34, 0 для 8)
    const START_IDX: usize;
    // Индекс, на котором реальные блоки заканчиваются (33 для 34, 8 для 8)
    const END_IDX: usize;
}

// Конфигурация для твоего основного мира
pub struct WorldChunk32;
impl ChunkConfig for WorldChunk32 {
    const REAL_SIZE: usize = 32;
    const VIRT_SIZE: usize = 34;
    const HAS_PADDING: bool = true;
    const START_IDX: usize = 1;
    const END_IDX: usize = 33;
}

// Конфигурация для твоего летящего корабля
pub struct PhysMiniChunk8;
impl ChunkConfig for PhysMiniChunk8 {
    const REAL_SIZE: usize = 8;
    const VIRT_SIZE: usize = 8;
    const HAS_PADDING: bool = false;
    const START_IDX: usize = 0;
    const END_IDX: usize = 8;
}

pub trait BitRow: Copy + Clone + Default {
    // Выставить бит по индексу X
    fn set_bit(&mut self, x: usize);

    // Операция row_now & !row_above & INTERNAL_MASK
    fn calculate_visible_faces(now: Self, adjacent: Self, mask: Self) -> Self;

    // Проверка, остались ли биты
    fn is_not_zero(&self) -> bool;

    // Логика для mesh_bitline (найти идущие подряд биты)
    // Внутри твоей mesh_bitline_34 ты скорее всего используешь .trailing_zeros() или побитовый сдвиг.
    // Трейт должен уметь отдавать информацию о битах.
    fn extract_next_line(&mut self) -> Option<(u32, u32)>; // (start_x, len_x)
}

impl BitRow for u8 {
    #[inline(always)]
    fn set_bit(&mut self, x: usize) {
        *self |= 1 << x;
    }

    #[inline(always)]
    fn calculate_visible_faces(now: Self, adjacent: Self, mask: Self) -> Self {
        now & !adjacent & mask
    }

    #[inline(always)]
    fn is_not_zero(&self) -> bool {
        *self != 0
    }

    #[inline(always)]
    fn extract_next_line(&mut self) -> Option<(u32, u32)> {
        if *self == 0 {
            return None;
        }

        let start = self.trailing_zeros();
        let length = (*self >> start).trailing_ones();

        let mask_to_clear = if length == 8 {
            u8::MAX
        } else {
            ((1u8 << length) - 1u8) << start
        };

        *self &= !mask_to_clear;

        Some((start, length))
    }
}

impl BitRow for u64 {
    #[inline(always)]
    fn is_not_zero(&self) -> bool {
        *self != 0
    }

    #[inline(always)]
    fn set_bit(&mut self, x: usize) {
        *self |= 1u64 << x;
    }

    #[inline(always)]
    fn calculate_visible_faces(now: Self, adjacent: Self, mask: Self) -> Self {
        now & !adjacent & mask
    }

    #[inline(always)]
    fn extract_next_line(&mut self) -> Option<(u32, u32)> {
        if *self == 0 {
            return None;
        }

        let start = self.trailing_zeros();
        let length = (*self >> start).trailing_ones();

        // БЕЗОПАСНАЯ МАСКА ОЧИСТКИ:
        let mask_to_clear = if length == 64 {
            u64::MAX
        } else {
            ((1u64 << length) - 1u64) << start
        };

        *self &= !mask_to_clear;

        Some((start, length))
    }
}

pub fn generate_mesh_generic<C, Row>(
    data: &[PrerenderBlock],
    internal_mask: Row,
) -> BlockIndexedPrimitive
where
    C: ChunkConfig,
    Row: BitRow,
{
    let mut vertices: Vec<BlockVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let mut present_materials = [false; 4096];
    let mut unique_materials = Vec::with_capacity(16);

    for block in data.iter() {
        let mat_id = block.get_type() as usize; // твои 12 бит
        if mat_id != 0 && !present_materials[mat_id] {
            present_materials[mat_id] = true;
            unique_materials.push(mat_id as u16);
        }
    }

    let mut vertex_counter: u32 = 0;

    for &current_mat in &unique_materials {
        // Размер векторов теперь берется из C::VIRT_SIZE (34 или 8)
        let mut y_layers = vec![vec![Row::default(); C::VIRT_SIZE]; C::VIRT_SIZE];
        let mut z_layers = vec![vec![Row::default(); C::VIRT_SIZE]; C::VIRT_SIZE];
        let mut x_layers = vec![vec![Row::default(); C::VIRT_SIZE]; C::VIRT_SIZE];

        // Заполнение слоев (код остается прежним, так как он просто идет по порядку data)
        let mut idx = 0;
        for y in 0..C::VIRT_SIZE {
            for z in 0..C::VIRT_SIZE {
                for x in 0..C::VIRT_SIZE {
                    let block = data[idx];
                    idx += 1;
                    if block.get_type().is_solid() && (block.get_type() as u16) == current_mat {
                        // y_layers[y][z].set_bit(x);
                        y_layers[y as usize][z as usize].set_bit(x);
                        x_layers[x as usize][y as usize].set_bit(z);
                        z_layers[z as usize][y as usize].set_bit(x);
                    }
                }
            }
        }
        let current_mat_u32 = current_mat as u32;

        // ==========================================
        // УНИВЕРСАЛЬНАЯ ГЕНЕРАЦИЯ ГРАНЕЙ
        // ==========================================
        // Идем от 0 до предпоследнего слоя в массиве
        for y in 0..(C::VIRT_SIZE - 1) {
            // А вот тут циклы по Z и X теперь берут границы из конфига!
            // Для мира это будет 1..33, для физики — 0..8
            for z in C::START_IDX..C::END_IDX {
                let row_now = y_layers[y][z];
                let row_above = y_layers[y + 1][z];

                let visible_faces_top =
                    Row::calculate_visible_faces(row_now, row_above, internal_mask);
                let visible_faces_bottom =
                    Row::calculate_visible_faces(row_above, row_now, internal_mask);

                // ВЕРХ (+Y)
                // Для мира: строим если y >= 1. Для физики без паддинга: строим ВСЕГДА (y >= 0)
                let can_draw_top = if C::HAS_PADDING { y >= 1 } else { true };

                if can_draw_top && visible_faces_top.is_not_zero() {
                    mesh_bitline_generic(
                        visible_faces_top,
                        z as u32,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        // ...
                        |start_x, len_x, z_val| {
                            // Математика вычисления координат для GPU.
                            // Если есть паддинг, мы вычитали 1. Если паддинга нет, вычитать НЕ НАДО!
                            let offset = C::START_IDX as u32;
                            let gpu_y = if C::HAS_PADDING {
                                y as u32
                            } else {
                                (y + 1) as u32
                            };
                            let gpu_z = (z_val as u32) - offset;
                            let x0 = (start_x as u32) - offset;
                            let x1 = (start_x as u32 + len_x as u32) - offset;

                            [
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_TOP, current_mat_u32),
                                BlockVertex::new(x1, gpu_y, gpu_z + 1, SIDE_TOP, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z + 1, SIDE_TOP, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_TOP, current_mat_u32),
                            ]
                        },
                    );
                }

                let can_draw_bottom = if C::HAS_PADDING {
                    y < (C::VIRT_SIZE - 2)
                } else {
                    true
                };

                if can_draw_bottom && visible_faces_bottom.is_not_zero() {
                    mesh_bitline_generic(
                        visible_faces_bottom,
                        z as u32,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, z_val| {
                            let offset = C::START_IDX as u32;
                            // Если есть паддинг, y соответствует базовой плоскости.
                            // Если паддинга нет, плоскость BOTTOM для верхнего блока находится на высоте y + 1
                            let gpu_y = if C::HAS_PADDING {
                                y as u32
                            } else {
                                (y + 1) as u32
                            };
                            let gpu_z = (z_val as u32) - offset;
                            let x0 = (start_x as u32) - offset;
                            let x1 = (start_x as u32 + len_x as u32) - offset;

                            [
                                BlockVertex::new(
                                    x1,
                                    gpu_y,
                                    gpu_z + 1,
                                    SIDE_BOTTOM,
                                    current_mat_u32,
                                ),
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_BOTTOM, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_BOTTOM, current_mat_u32),
                                BlockVertex::new(
                                    x0,
                                    gpu_y,
                                    gpu_z + 1,
                                    SIDE_BOTTOM,
                                    current_mat_u32,
                                ),
                            ]
                        },
                    );
                }
            }
        }

        for x in 0..(C::VIRT_SIZE - 1) {
            for y in C::START_IDX..C::END_IDX {
                let row_now = x_layers[x as usize][y as usize];
                let row_next = x_layers[(x + 1) as usize][y as usize];

                let visible_faces_east =
                    Row::calculate_visible_faces(row_now, row_next, internal_mask);
                let visible_faces_west =
                    Row::calculate_visible_faces(row_next, row_now, internal_mask);

                // ВОСТОК (+X)
                let can_draw_east = if C::HAS_PADDING { x >= 1 } else { true };

                if can_draw_east && visible_faces_east.is_not_zero() {
                    mesh_bitline_generic(
                        visible_faces_east,
                        y as u32,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_z, len_z, y_val| {
                            let offset = C::START_IDX as u32;
                            let gpu_x = if C::HAS_PADDING {
                                x as u32
                            } else {
                                (x + 1) as u32
                            };
                            let gpu_y = (y_val as u32) - offset;
                            let z0 = (start_z as u32) - offset;
                            let z1 = (start_z as u32 + len_z as u32) - offset;
                            [
                                BlockVertex::new(gpu_x, gpu_y, z1, SIDE_EAST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z1, SIDE_EAST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z0, SIDE_EAST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y, z0, SIDE_EAST, current_mat_u32),
                            ]
                        },
                    );
                }

                // ЗАПАД (-X)
                let can_draw_west = if C::HAS_PADDING {
                    x < (C::VIRT_SIZE - 2)
                } else {
                    true
                };

                if can_draw_west && visible_faces_west.is_not_zero() {
                    mesh_bitline_generic(
                        visible_faces_west,
                        y as u32,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_z, len_z, y_val| {
                            let offset = C::START_IDX as u32;
                            let gpu_x = if C::HAS_PADDING {
                                x as u32
                            } else {
                                (x + 1) as u32
                            };
                            let gpu_y = (y_val as u32) - offset;
                            let z0 = (start_z as u32) - offset;
                            let z1 = (start_z as u32 + len_z as u32) - offset;
                            [
                                BlockVertex::new(gpu_x, gpu_y + 1, z1, SIDE_WEST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y, z1, SIDE_WEST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y, z0, SIDE_WEST, current_mat_u32),
                                BlockVertex::new(gpu_x, gpu_y + 1, z0, SIDE_WEST, current_mat_u32),
                            ]
                        },
                    );
                }
            }
        }

        for z in 0..(C::VIRT_SIZE - 1) {
            for y in C::START_IDX..C::END_IDX {
                let row_now = z_layers[z as usize][y as usize];
                let row_next = z_layers[(z + 1) as usize][y as usize];

                let visible_faces_south =
                    Row::calculate_visible_faces(row_now, row_next, internal_mask);
                let visible_faces_north =
                    Row::calculate_visible_faces(row_next, row_now, internal_mask);

                // ЮГ (+Z)
                let can_draw_south = if C::HAS_PADDING { z >= 1 } else { true };

                if can_draw_south && visible_faces_south.is_not_zero() {
                    mesh_bitline_generic(
                        visible_faces_south,
                        y as u32,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, y_val| {
                            let offset = C::START_IDX as u32;
                            let gpu_z = if C::HAS_PADDING {
                                z as u32
                            } else {
                                (z + 1) as u32
                            };
                            let gpu_y = (y_val as u32) - offset;
                            let x0 = (start_x as u32) - offset;
                            let x1 = (start_x as u32 + len_x as u32) - offset;
                            [
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_SOUTH, current_mat_u32),
                                BlockVertex::new(x0, gpu_y + 1, gpu_z, SIDE_SOUTH, current_mat_u32),
                                BlockVertex::new(x1, gpu_y + 1, gpu_z, SIDE_SOUTH, current_mat_u32),
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_SOUTH, current_mat_u32),
                            ]
                        },
                    );
                }

                // СЕВЕР (-Z)
                let can_draw_north = if C::HAS_PADDING {
                    z < (C::VIRT_SIZE - 2)
                } else {
                    true
                };

                if can_draw_north && visible_faces_north.is_not_zero() {
                    mesh_bitline_generic(
                        visible_faces_north,
                        y as u32,
                        &mut vertex_counter,
                        &mut vertices,
                        &mut indices,
                        |start_x, len_x, y_val| {
                            let offset = C::START_IDX as u32;
                            let gpu_z = if C::HAS_PADDING {
                                z as u32
                            } else {
                                (z + 1) as u32
                            };
                            let gpu_y = (y_val as u32) - offset;
                            let x0 = (start_x as u32) - offset;
                            let x1 = (start_x as u32 + len_x as u32) - offset;
                            [
                                BlockVertex::new(x1, gpu_y, gpu_z, SIDE_NORTH, current_mat_u32),
                                BlockVertex::new(x1, gpu_y + 1, gpu_z, SIDE_NORTH, current_mat_u32),
                                BlockVertex::new(x0, gpu_y + 1, gpu_z, SIDE_NORTH, current_mat_u32),
                                BlockVertex::new(x0, gpu_y, gpu_z, SIDE_NORTH, current_mat_u32),
                            ]
                        },
                    );
                }
            }
        }
    }
    // ...
    BlockIndexedPrimitive::new(vertices, indices)
}

pub fn mesh_bitline_generic<Row, F>(
    mut visible_faces: Row, // Дженерик тип битовой строки (u64 или u8)
    cross_coord: u32,
    vertex_counter: &mut u32,
    vertices: &mut Vec<BlockVertex>,
    indices: &mut Vec<u32>,
    mut build_quad: F,
) where
    Row: BitRow,
    F: FnMut(u32, u32, u32) -> [BlockVertex; 4],
{
    // Быстрый выход, если строка пустая
    if !visible_faces.is_not_zero() {
        return;
    }

    // Цикл крутится, пока метод extract_next_line выдает нам линии полигонов
    while let Some((start, length)) = visible_faces.extract_next_line() {
        // Вызываем переданное замыкание для сборки квада
        let packed_vertices = build_quad(start, length, cross_coord);

        // Закидываем вершины в общий буфер чанка
        vertices.extend_from_slice(&packed_vertices);

        // Строим два треугольника (индексы квада)
        let vc = *vertex_counter;
        indices.extend_from_slice(&[vc + 0, vc + 1, vc + 2, vc + 0, vc + 2, vc + 3]);

        // Сдвигаем счетчик для следующего квада
        *vertex_counter += 4;
    }
}
