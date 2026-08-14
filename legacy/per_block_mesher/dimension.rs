use std::collections::HashMap;
use std::time::Instant;

use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::types::Vertex;

use crate::chunk::{CHUNK_ARRAY_LEN, CHUNK_SIZE, Chunk};
use crate::coordinates::{ChunkCoords, GlobalCoords};
use crate::new_mesher::{generate_mesh, into_new_array, into_prerender_array};
use crate::types::blocks::block::BlockType;
use crate::types::coordinates::{LocalCoords, LocalCoordsType};

pub struct Dimension {
    chunks: HashMap<ChunkCoords, Chunk>,
    dirty_chunks: Vec<ChunkCoords>,
}

pub const BORDER_MIN: u32 = 0;
pub const BORDER_MAX: u32 = 31;

impl Dimension {
    pub fn new() -> Self {
        let start = Instant::now();
        let mut chunks: HashMap<ChunkCoords, Chunk> = Default::default();
        let mut dirty_chunks = Vec::new();
        for x in 0..=2 {
            for y in 0..=2 {
                for z in 0..=2 {
                    chunks.insert(
                        (x, y, z).into(),
                        // Chunk::from_block_as_grid(BlockType::Dirt, 2),
                        Chunk::from_block(BlockType::Dirt),
                    );
                    dirty_chunks.push((x, y, z).into());
                }
            }
        }
        println!(
            "Генерация {} чанков: {} ms",
            chunks.len(),
            start.elapsed().as_millis()
        );
        Self {
            chunks,
            dirty_chunks,
        }
    }

    pub fn iter_chunks(&self) -> impl Iterator<Item = (&ChunkCoords, &Chunk)> {
        self.chunks.iter()
    }

    pub fn get_chunk(&self, chunk_coordinates: ChunkCoords) -> Option<&Chunk> {
        self.chunks.get(&chunk_coordinates)
    }

    pub fn get_chunk_or_create(&mut self, chunk_coordinates: ChunkCoords) -> &Chunk {
        self.get_chunk_mut_or_create(chunk_coordinates)
    }

    pub fn get_chunk_mut(&mut self, chunk_coordinates: ChunkCoords) -> Option<&mut Chunk> {
        self.chunks.get_mut(&chunk_coordinates)
    }

    pub fn get_chunk_mut_or_create(&mut self, chunk_coordinates: ChunkCoords) -> &mut Chunk {
        self.chunks
            .entry(chunk_coordinates)
            .or_insert(Chunk::from_block(BlockType::Air))
    }

    fn dirt_chunk(&mut self, chunk_coordinates: ChunkCoords) {
        self.dirty_chunks.push(chunk_coordinates);
    }

    pub fn update_chunk_meshes(&mut self, renderer: &mut Renderer) {
        if self.dirty_chunks.len() > 0 {
            let start = Instant::now();
            while self.dirty_chunks.len() > 0 {
                let dirty_chunk_coords = self.dirty_chunks.pop().unwrap();

                let chunk_mesh = self.get_chunk_mesh(dirty_chunk_coords.clone());

                let chunk = self.get_chunk_mut(dirty_chunk_coords).unwrap();

                if let Some(old_id) = chunk.vram_slot_id {
                    renderer.unload_mesh(old_id);
                }

                let id = renderer
                    .load_mesh(chunk_mesh, dirty_chunk_coords.get_model_mat())
                    .unwrap();
                chunk.vram_slot_id = Some(id);
            }
            println!("Обновление чанков: {} ms", start.elapsed().as_millis());
        }
    }

    pub fn get_chunk_mesh(&self, chunk_coordinates: ChunkCoords) -> (Vec<Vertex>, Vec<u32>) {
        let central_chunk = self.chunks.get(&chunk_coordinates).unwrap();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut vertex_count: u32 = 0;

        // let mut check_border_voxel_ = |local_coords: LocalCoords| {
        //     let coordinates = (chunk_coordinates.clone(), local_coords).into();
        //     check_border_voxel(
        //         &mut vertices,
        //         &mut indices,
        //         &mut vertex_count,
        //         &central_chunk,
        //         coordinates,
        //         &self,
        //     );
        // };

        // // === 1. ГРАНИЦА -X (Левая стена, x = 31) ===
        // // === 2. ГРАНИЦА +X (Правая стена, x = 31) ===
        // for x in [BORDER_MIN, BORDER_MAX] {
        //     for y in 0..CHUNK_SIZE {
        //         for z in 0..CHUNK_SIZE {
        //             check_border_voxel_(LocalCoords::from((x, y, z)));
        //         }
        //     }
        // }

        // // === 3. ГРАНИЦА -Y (Пол, y = 0) ===
        // // === 4. ГРАНИЦА +Y (Потолок, y = 31) ===
        // for y in [BORDER_MIN, BORDER_MAX] {
        //     for x in 0..CHUNK_SIZE {
        //         for z in 0..CHUNK_SIZE {
        //             check_border_voxel_(LocalCoords::from((x, y, z)));
        //         }
        //     }
        // }

        // // === 5. ГРАНИЦА -Z (Задняя стена, z = 0) ===
        // // === 6. ГРАНИЦА +Z (Передняя стена, z = 31) ===
        // for z in [BORDER_MIN, BORDER_MAX] {
        //     for y in 0..CHUNK_SIZE {
        //         for x in 0..CHUNK_SIZE {
        //             check_border_voxel_(LocalCoords::from((x, y, z)));
        //         }
        //     }
        // }

        // for y in 1..(CHUNK_SIZE - 1) {
        //     for z in 1..(CHUNK_SIZE - 1) {
        //         for x in 1..(CHUNK_SIZE - 1) {
        //             check_inner_voxel(
        //                 &mut vertices,
        //                 &mut indices,
        //                 &mut vertex_count,
        //                 &central_chunk.data,
        //                 x,
        //                 y,
        //                 z,
        //             );
        //         }
        //     }
        // }

        let new_arr = into_new_array(&central_chunk.data);
        let prerender_arr = into_prerender_array(&new_arr);
        generate_mesh(&prerender_arr)

        // (vertices, indices)
    }

    pub fn get_block(&self, coords: GlobalCoords) -> BlockType {
        match self.get_chunk(coords.get_chunk()) {
            Some(chunk) => chunk.get_block(coords.get_local()),
            None => BlockType::Air,
        }
    }

    pub fn set_block(&mut self, coords: GlobalCoords, block: BlockType) {
        let chunk_coords = coords.get_chunk();
        let chunk = self.get_chunk_mut_or_create(chunk_coords);
        chunk.set_block(coords.get_local(), block);
        self.dirt_chunk(chunk_coords);
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

// pub fn is_neighbor_blocked(
//     nx: LocalCoordsType,
//     ny: LocalCoordsType,
//     nz: LocalCoordsType,
//     data: &[BlockType; CHUNK_ARRAY_LEN],
// ) -> bool {
//     if !LocalCoords::from((nx, ny, nz)).in_chunk() {
//         return false;
//     }
//     let pos: LocalCoords = (nx, ny, nz).into();
//     pos.in_chunk() && data[usize::from(pos)].is_solid()
// }

// pub fn check_inner_voxel(
//     vertices: &mut Vec<Vertex>,
//     indices: &mut Vec<u32>,
//     vert_count: &mut u32,
//     data: &[BlockType; CHUNK_ARRAY_LEN],
//     x: LocalCoordsType,
//     y: LocalCoordsType,
//     z: LocalCoordsType,
// ) {
//     let mut vertex_count: u32 = vert_count.clone();

//     let pos: LocalCoords = (x, y, z).into();

//     if !(pos.in_chunk() && data[usize::from(pos)].is_solid()) {
//         return;
//     }

//     let fx = x as f32;
//     let fy = y as f32;
//     let fz = z as f32;

//     // --- 1. ВЕРХНЯЯ ГРАНЬ (+Y) ---
//     if !is_neighbor_blocked(x, y + 1, z, data) {
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz + 1.0],
//             uv: [0.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz],
//             uv: [1.0, 0.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 2. НИЖНЯЯ ГРАНЬ (-Y) ---
//     if !is_neighbor_blocked(x, y - 1, z, data) {
//         vertices.push(Vertex {
//             position: [fx, fy, fz],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy, fz + 1.0],
//             uv: [0.0, 1.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 3,
//             vertex_count + 2,
//             vertex_count + 0,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 3. ПЕРЕДНЯЯ ГРАНЬ (+Z) ---
//     if !is_neighbor_blocked(x, y, z + 1, data) {
//         vertices.push(Vertex {
//             position: [fx, fy, fz + 1.0],
//             uv: [0.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz + 1.0],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz + 1.0],
//             uv: [0.0, 0.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 3,
//             vertex_count + 2,
//             vertex_count + 0,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 4. ЗАДНЯЯ ГРАНЬ (-Z) ---
//     if !is_neighbor_blocked(x, y, z - 1, data) {
//         vertices.push(Vertex {
//             position: [fx, fy, fz],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz],
//             uv: [0.0, 1.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 5. ПРАВАЯ ГРАНЬ (+X) ---
//     if !is_neighbor_blocked(x + 1, y, z, data) {
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz + 1.0],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz + 1.0],
//             uv: [0.0, 1.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 6. ЛЕВАЯ ГРАНЬ (-X) ---
//     if !is_neighbor_blocked(x - 1, y, z, data) {
//         vertices.push(Vertex {
//             position: [fx, fy, fz],
//             uv: [0.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz + 1.0],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz],
//             uv: [0.0, 0.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }
//     *vert_count = vertex_count;
// }

// pub fn check_border_voxel(
//     vertices: &mut Vec<Vertex>,
//     indices: &mut Vec<u32>,
//     vert_count: &mut u32,
//     central_chunk: &Chunk,
//     coordinates: GlobalCoords,
//     dimension: &Dimension,
// ) {
//     let (x, y, z) = coordinates.clone().into();
//     let (lx, ly, lz) = coordinates.get_local().into();
//     let mut vertex_count: u32 = vert_count.clone();
//     if !central_chunk.get_block(coordinates.get_local()).is_solid() {
//         return;
//     }

//     let fx = lx as f32;
//     let fy = ly as f32;
//     let fz = lz as f32;

//     // --- 1. ВЕРХНЯЯ ГРАНЬ (+Y) ---
//     if !dimension
//         .get_block(GlobalCoords::from((x, y + 1, z)))
//         .is_solid()
//     {
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz + 1.0],
//             uv: [0.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz],
//             uv: [1.0, 0.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 2. НИЖНЯЯ ГРАНЬ (-Y) ---
//     if !dimension
//         .get_block(GlobalCoords::from((x, y - 1, z)))
//         .is_solid()
//     {
//         vertices.push(Vertex {
//             position: [fx, fy, fz],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy, fz + 1.0],
//             uv: [0.0, 1.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 3,
//             vertex_count + 2,
//             vertex_count + 0,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 3. ПЕРЕДНЯЯ ГРАНЬ (+Z) ---
//     if !dimension
//         .get_block(GlobalCoords::from((x, y, z + 1)))
//         .is_solid()
//     {
//         vertices.push(Vertex {
//             position: [fx, fy, fz + 1.0],
//             uv: [0.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz + 1.0],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz + 1.0],
//             uv: [0.0, 0.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 3,
//             vertex_count + 2,
//             vertex_count + 0,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 4. ЗАДНЯЯ ГРАНЬ (-Z) ---
//     if !dimension
//         .get_block(GlobalCoords::from((x, y, z - 1)))
//         .is_solid()
//     {
//         vertices.push(Vertex {
//             position: [fx, fy, fz],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz],
//             uv: [0.0, 1.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 5. ПРАВАЯ ГРАНЬ (+X) ---
//     if !dimension
//         .get_block(GlobalCoords::from((x + 1, y, z)))
//         .is_solid()
//     {
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy + 1.0, fz + 1.0],
//             uv: [0.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx + 1.0, fy, fz + 1.0],
//             uv: [0.0, 1.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }

//     // --- 6. ЛЕВАЯ ГРАНЬ (-X) ---
//     if !dimension
//         .get_block(GlobalCoords::from((x - 1, y, z)))
//         .is_solid()
//     {
//         vertices.push(Vertex {
//             position: [fx, fy, fz],
//             uv: [0.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy, fz + 1.0],
//             uv: [1.0, 1.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz + 1.0],
//             uv: [1.0, 0.0],
//         });
//         vertices.push(Vertex {
//             position: [fx, fy + 1.0, fz],
//             uv: [0.0, 0.0],
//         });

//         indices.extend_from_slice(&[
//             vertex_count + 0,
//             vertex_count + 2,
//             vertex_count + 1,
//             vertex_count + 0,
//             vertex_count + 3,
//             vertex_count + 2,
//         ]);
//         vertex_count += 4;
//     }
//     *vert_count = vertex_count;
// }
