use glam::{Mat4, Vec3};

use super::core::ChunkCoords;
use crate::chunk::CHUNK_SIZE;

impl ChunkCoords {
    pub fn get_model_mat(self) -> [[f32; 4]; 4] {
        Mat4::from_translation(Vec3::new(
            (self.x * CHUNK_SIZE as i32) as f32,
            (self.y * CHUNK_SIZE as i32) as f32,
            (self.z * CHUNK_SIZE as i32) as f32,
        ))
        .to_cols_array_2d()
    }
}
