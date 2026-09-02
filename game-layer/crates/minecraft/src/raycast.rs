use glam::Vec3;
use lib_core::math::vectors::custom::PrecisePositionC32;

use crate::types::coordinates::core::{ChunkCoords, GlobalCoords, LocalCoords};
use crate::types::dimension::Dimension;

pub struct RaycastResult {
    pub target_block: GlobalCoords,
    pub previous_block: GlobalCoords,
}

pub fn raycast(
    dimension: &Dimension,
    origin: PrecisePositionC32,
    direction: Vec3,
    max_distance: f32,
) -> Option<RaycastResult> {
    // Если будет криво определяться prev_block. Можно в условии с установкой current_distance = t_max.x; также устанавливать вектор, который относительно current_in_chunk покажет предыдущий блок.
    let dir = direction.normalize();

    let mut current_in_chunk = origin.in_chunk.floor_cw().as_i32();
    let step = Vec3::select(dir.cmpgt(Vec3::ZERO), Vec3::ONE, Vec3::NEG_ONE).as_ivec3();

    let delta = (Vec3::ONE / dir).abs();

    let mut voxel_fract = Vec3::from(origin.in_chunk).fract();
    voxel_fract = Vec3::select(
        voxel_fract.cmplt(Vec3::ZERO),
        voxel_fract + Vec3::ONE,
        voxel_fract,
    );
    let mut t_max = Vec3::select(
        dir.cmpgt(Vec3::ZERO),
        (Vec3::ONE - voxel_fract) * delta,
        voxel_fract * delta,
    );

    let mut prev_in_chunk = current_in_chunk;

    let mut current_distance: f32 = 0.0;

    let chunk_shift_i64 = origin.chunk.as_i64().shl_all(5);

    while current_distance < max_distance {
        let current_global = GlobalCoords::from(chunk_shift_i64 + current_in_chunk.as_i64());
        let block = dimension.get_block_loaded(current_global);
        if block.get_type().is_solid() {
            return Some(RaycastResult {
                target_block: current_global,
                previous_block: GlobalCoords::from(chunk_shift_i64 + prev_in_chunk.as_i64()),
            });
        }

        prev_in_chunk = current_in_chunk;

        if t_max.x < t_max.y {
            if t_max.x < t_max.z {
                current_distance = t_max.x;
                t_max.x += delta.x;
                current_in_chunk.x += step.x;
            } else {
                current_distance = t_max.z;
                t_max.z += delta.z;
                current_in_chunk.z += step.z;
            }
        } else {
            if t_max.y < t_max.z {
                current_distance = t_max.y;
                t_max.y += delta.y;
                current_in_chunk.y += step.y;
            } else {
                current_distance = t_max.z;
                t_max.z += delta.z;
                current_in_chunk.z += step.z;
            }
        }
    }

    None
}
