use glam::{Mat3, Quat, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;

use crate::physics_world::physics_construction::PhysicsConstruction;
use crate::physics_world::utils;
use crate::types::coordinates::core::MiniChunkCoords;
use crate::utils::raycast::{RaycastResult, raycast_phys};

#[derive(Default)]
pub struct RaycastModule;

impl RaycastModule {
    pub fn raycast_physics(
        &self,
        construct_data: &Vec<PhysicsConstruction>,
        positions: &Vec<PrecisePositionC32>,
        orientations: &Vec<Quat>,
        origin: PrecisePositionC32,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<(RaycastResult<MiniChunkCoords>, usize)> {
        let mut closest_entity: Option<(RaycastResult<MiniChunkCoords>, usize)> = None;
        let mut min_voxel_distance = max_dist; // Сравниваем именно РЕАЛЬНЫЕ дистанции до блоков

        for (idx, (position, data)) in positions.iter().zip(construct_data.iter()).enumerate() {
            // 1. Находим f32 сдвиг центра масс объекта относительно игрока
            let relative_chunk_pos = position.chunk - origin.chunk;
            let relative_pos: Vec3 =
                (relative_chunk_pos.as_f32() * 32.0 + position.in_chunk - origin.in_chunk).into();

            // Достаем текущий кватернион поворота конструкта из ECS
            let object_rot: Quat = orientations[idx];

            // ====================================================================
            // РАСЧЕТ ВНЕШНЕЙ МИРОВОЙ КОРОБКИ (World AABB) С УЧЕТОМ ВРАЩЕНИЯ:
            // ====================================================================
            let rot_mat = Mat3::from_quat(object_rot);

            // Полуразмеры (радиусы) локальной коробки конструкта
            let local_extents = (data.mcr_box_max - data.mcr_box_min) * 0.5;
            // Центр локальной коробки относительно физического центра масс
            let local_center = (data.mcr_box_min + data.mcr_box_max) * 0.5;

            // Поворачиваем локальный центр коробки и смещаем его в пространство игрока
            let world_center = rot_mat * local_center + relative_pos;

            // Проектируем повернутые полуразмеры коробки на глобальные оси мира (X, Y, Z)
            let new_extents = Vec3::new(
                rot_mat.row(0).abs().dot(local_extents),
                rot_mat.row(1).abs().dot(local_extents),
                rot_mat.row(2).abs().dot(local_extents),
            );

            // Финальная выровненная по осям мира коробка, "окутывающая" повернутый объект
            let world_box_min = world_center - new_extents;
            let world_box_max = world_center + new_extents;

            // ====================================================================
            // ШИРОКАЯ ФАЗА (Broadphase)
            // ====================================================================
            // Проверяем честный глобальный луч игрока (из Vec3::ZERO) против мировой коробки
            if let Some(_box_dist) =
                utils::ray_intersects_aabb(Vec3::ZERO, dir, world_box_min, world_box_max)
            {
                // ====================================================================
                // УЗКАЯ ФАЗА (Narrowphase)
                // ====================================================================
                // Переводим луч в локальные координаты конструкта
                let relative_ray_origin = -relative_pos;
                let inv_rot = object_rot.inverse();
                let local_ray_origin = inv_rot * relative_ray_origin;
                let local_ray_dir = inv_rot * dir;

                // Вызываем рейкаст по сетке 8х8х8 (твой алгоритм)
                if let Some(result) =
                    self.raycast_voxels_inside(construct_data, idx, local_ray_origin, local_ray_dir)
                {
                    // Если реальный блок внутри этого конструкта ближе, чем у предыдущих
                    if result.distance < min_voxel_distance {
                        min_voxel_distance = result.distance;
                        closest_entity = Some((result, idx));
                    }
                }
            }
        }

        // Возвращаем конструкт, у которого НАСТОЯЩИЙ БЛОК оказался ближе всего к лицу игрока
        closest_entity
    }

    pub fn raycast_voxels_inside(
        &self,
        construct_data: &Vec<PhysicsConstruction>,
        idx: usize,
        local_ray_origin: Vec3,
        dir: Vec3,
    ) -> Option<RaycastResult<MiniChunkCoords>> {
        let local_mass_center = construct_data[idx].local_mass_center;

        let grid_ray_origin = local_ray_origin + local_mass_center;

        let grid_ray_dir = dir;

        raycast_phys(
            &construct_data[idx],
            grid_ray_origin.into(),
            grid_ray_dir,
            8.0,
        )
    }
}
