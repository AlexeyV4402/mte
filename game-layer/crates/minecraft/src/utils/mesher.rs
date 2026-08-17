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

pub fn into_prerender_array(data: &[Block; CHUNK_ARRAY_LEN]) -> [PrerenderBlock; CHUNK_ARRAY_LEN] {
    let mut result = [PrerenderBlock::default(); CHUNK_ARRAY_LEN];
    for (i, block) in data.iter().enumerate() {
        result[i] = PrerenderBlock::from_block(block.clone());
    }
    result
}

const INTERNAL_MASK: u64 = 0x1_FFFF_FFFE;

pub fn generate_mesh(data: &[PrerenderBlock; CHUNK_ARRAY_LEN]) -> BlockIndexedPrimitive {
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
                    // Читаем массив data строго последовательно, idx увеличивается от 0 до 39303
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
                    mesh_bitline(
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
                    mesh_bitline(
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
                    mesh_bitline(
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
                    mesh_bitline(
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
                    mesh_bitline(
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
                    mesh_bitline(
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

fn mesh_bitline<F>(
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

        if visible_faces == 1 {
            println!("Итерация");
        }

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
