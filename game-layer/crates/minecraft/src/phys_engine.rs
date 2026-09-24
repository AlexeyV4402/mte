use std::mem;
use std::time::Duration;

use glam::{Mat3, Mat4, Quat, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::indirect_buffer_manager::PhysObjectGpuHandle;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use lib_renderer::renderer::block_grid_renderer::types::BlockVertex;

use crate::raycast::{RaycastResult, raycast_phys};
use crate::types::blocks::block::Block;
use crate::types::coordinates::core::{GlobalCoords, MiniChunkCoords};
use crate::types::world::PhysicsEvent;
use crate::utils::mesher::{
    PhysMiniChunk8, SIDE_BOTTOM, SIDE_EAST, SIDE_NORTH, SIDE_SOUTH, SIDE_TOP, SIDE_WEST, generate_mesh_generic, into_prerender_array
};

#[derive(Default)]
pub struct PhysicsEngine {
    pub data: ECS,
}

const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

impl PhysicsEngine {
    pub fn read_queue(&mut self, queue: &mut Vec<PhysicsEvent>) {
        queue.drain(..).for_each(|event| match event {
            PhysicsEvent::PlaceBlock(coords, block) => {
                self.data.push_default(coords, block);
            }
        });
    }

    pub fn set_block(&mut self, idx: usize, coords: MiniChunkCoords, block: Block) {
        self.data.construct_data[idx].blocks[usize::from(coords)] = block;
        let old_mass_center = self.data.construct_data[idx].local_mass_center;

        let exists = self.data.construct_data[idx].update_box();

        let new_mass_center = self.data.construct_data[idx].local_mass_center;
        self.data.positions[idx].in_chunk += (new_mass_center - old_mass_center).into();
        self.data.show_queue.push(idx);
    }

    pub fn update_meshes(&mut self, renderer: &mut VkBackend) {
        if self.data.show_queue.len() > 0 {
            let mut queue = mem::replace(&mut self.data.show_queue, Vec::with_capacity(32));
            queue.drain(..).for_each(|idx| {
                let mesh = self.data.construct_data[idx].get_mesh();
                if self.data.object_handles.len() > idx {
                    if let Some(old_handle) = self.data.object_handles[idx] {
                        renderer.unload_phys_object(old_handle);
                    }
                } else {
                    self.data.object_handles.push(None);
                }

                self.data.object_handles[idx] =
                    renderer.load_phys_object(mesh, self.get_vector(idx), self.get_matrix(idx));
            });
        }
        let mut queue = mem::replace(&mut self.data.hide_queue, Vec::with_capacity(32));
        queue.drain(..).for_each(|idx| {
            if let Some(handle) = self.data.object_handles[idx] {
                renderer.unload_phys_object(handle);
                self.data.object_handles[idx] = None;
            }
        });
        let mut queue = mem::replace(&mut self.data.update_queue, Vec::with_capacity(32));
        queue.drain(..).for_each(|idx| {
            if let Some(handle) = self.data.object_handles[idx] {
                renderer.update_phys_object(handle, self.get_vector(idx), self.get_matrix(idx));
            }
        });
    }

    pub fn get_matrix(&self, idx: usize) -> [[f32; 4]; 4] {
        // 1. Получаем физическую позицию центра масс внутри чанка мира
        let translation: Vec3 = self.data.positions[idx].in_chunk.into();

        // 2. Получаем физическую ориентацию (поворот)
        let rotation: Quat = self.data.orientations[idx];

        // 3. Получаем локальное смещение центра масс относительно нулевого угла сетки 8х8х8
        let local_mass_center: Vec3 = self.data.construct_data[idx].local_mass_center;

        // 4. Строим матрицу:
        //    Сначала сдвигаем меш назад на (-local_mass_center), чтобы совместить центр масс с (0,0,0).
        //    Затем вращаем кватернионом вокруг (0,0,0).
        //    Затем переносим повернутый объект в нужные f32-координаты чанка.
        let model_matrix = Mat4::from_rotation_translation(rotation, translation)
            * Mat4::from_translation(-local_mass_center);

        // 5. Превращаем в сырой массив для Vulkan-рендерера
        model_matrix.to_cols_array_2d()
    }

    pub fn get_vector(&self, idx: usize) -> [i32; 4] {
        self.data.positions[idx].chunk.to_vec4_left().to_array()
    }

    pub fn update(&mut self, delta_time: Duration) {
        let delta_time = delta_time.as_secs_f32();

        self.system_apply_gravity();

        // self.system_apply_grav_gun(player_target);
        // self.system_apply_aerodynamics();

        self.system_integrate_velocities(delta_time);

        self.system_integrate_orientations(delta_time);
        self.system_integrate_positions(delta_time);

        self.system_clear_forces();
    }

    #[inline(always)]
    fn system_apply_gravity(&mut self) {
        let gravity = Vec3::new(0.0, -9.81, 0.0);

        for (force, &inv_mass) in self
            .data
            .forces
            .iter_mut()
            .zip(self.data.inverse_masses.iter())
        {
            if inv_mass > 0.0 {
                let mass = 1.0 / inv_mass;
                *force += gravity * mass;
            }
        }
    }

    #[inline(always)]
    fn system_integrate_velocities(&mut self, delta_time: f32) {
        let loops = self.data.velocities.len();
        for i in 0..loops {
            let inv_mass = self.data.inverse_masses[i];
            if inv_mass > 0.0 {
                self.data.accelerations[i] = self.data.forces[i] * inv_mass;
                self.data.velocities[i] += self.data.accelerations[i] * delta_time;
                self.data.update_queue.push(i);
            }
        }
    }

    #[inline(always)]
    fn system_integrate_positions(&mut self, delta_time: f32) {
        for (pos, velocity) in self
            .data
            .positions
            .iter_mut()
            .zip(self.data.velocities.iter())
        {
            let displacement = *velocity * delta_time;
            if (pos.chunk.as_f32() * 32.0 + pos.in_chunk + displacement.into()).all_le(0.5) {
                continue;
            }

            pos.in_chunk += displacement.into();

            // Ваша крутая векторизованная нормализация чанков
            pos.normalize();
        }
    }

    #[inline(always)]
    fn system_integrate_orientations(&mut self, delta_time: f32) {
        for (quat, velocity) in self
            .data
            .orientations
            .iter_mut()
            .zip(self.data.angular_velocities.iter())
        {
            let angular_velocity = velocity;
            if angular_velocity != &Vec3::ZERO {
                let rotation_delta = Quat::from_axis_angle(
                    angular_velocity.normalize(),
                    angular_velocity.length() * delta_time,
                );
                *quat = (rotation_delta * (*quat)).normalize();
            }
        }
    }

    #[inline(always)]
    fn system_clear_forces(&mut self) {
        for force in self.data.forces.iter_mut() {
            *force = Vec3::ZERO;
        }
    }

    pub fn raycast_physics(
        &self,
        origin: PrecisePositionC32,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<(RaycastResult<MiniChunkCoords>, usize)> {
        let mut closest_entity: Option<(RaycastResult<MiniChunkCoords>, usize)> = None;
        let mut min_voxel_distance = max_dist; // Сравниваем именно РЕАЛЬНЫЕ дистанции до блоков

        for (idx, (position, data)) in self
            .data
            .positions
            .iter()
            .zip(self.data.construct_data.iter())
            .enumerate()
        {
            // 1. Находим f32 сдвиг центра масс объекта относительно игрока
            let relative_chunk_pos = position.chunk - origin.chunk;
            let relative_pos: Vec3 =
                (relative_chunk_pos.as_f32() * 32.0 + position.in_chunk - origin.in_chunk).into();

            // Достаем текущий кватернион поворота конструкта из ECS
            let object_rot: Quat = self.data.orientations[idx];

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
                Self::ray_intersects_aabb(Vec3::ZERO, dir, world_box_min, world_box_max)
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
                    self.raycast_voxels_inside(idx, local_ray_origin, local_ray_dir)
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
        idx: usize,
        local_ray_origin: Vec3,
        dir: Vec3,
    ) -> Option<RaycastResult<MiniChunkCoords>> {
        let local_mass_center = self.data.construct_data[idx].local_mass_center;

        let grid_ray_origin = local_ray_origin + local_mass_center;

        let grid_ray_dir = dir;

        raycast_phys(
            &self.data.construct_data[idx],
            grid_ray_origin.into(),
            grid_ray_dir,
            8.0,
        )
    }

    pub fn ray_intersects_aabb(
        ray_origin: Vec3,
        ray_dir: Vec3,
        box_min: Vec3,
        box_max: Vec3,
    ) -> Option<f32> {
        // 1. Считаем инверсию направления луча (1.0 / dir), чтобы заменить дорогое деление на умножение.
        // Если какая-то координата dir равна 0.0 (луч параллелен оси), Rust вернет Infinity.
        // Математика ниже это корректно обработает без падения программы.
        let inv_dir = Vec3::new(1.0 / ray_dir.x, 1.0 / ray_dir.y, 1.0 / ray_dir.z);

        // 2. Находим точки пересечения с плоскостями X
        let t1 = (box_min.x - ray_origin.x) * inv_dir.x;
        let t2 = (box_max.x - ray_origin.x) * inv_dir.x;
        // Определяем, где был вход, а где выход по оси X
        let tmin_x = t1.min(t2);
        let tmax_x = t1.max(t2);

        // 3. Находим точки пересечения с плоскостями Y
        let t3 = (box_min.y - ray_origin.y) * inv_dir.y;
        let t4 = (box_max.y - ray_origin.y) * inv_dir.y;
        let tmin_y = t3.min(t4);
        let tmax_y = t3.max(t4);

        // 4. Находим точки пересечения с плоскостями Z
        let t5 = (box_min.z - ray_origin.z) * inv_dir.z;
        let t6 = (box_max.z - ray_origin.z) * inv_dir.z;
        let tmin_z = t5.min(t6);
        let tmax_z = t5.max(t6);

        // 5. Находим общее время входа (самый поздний вход из всех трех осей)
        // и общее время выхода (самый ранний выход из всех трех осей)
        let tmin = tmin_x.max(tmin_y).max(tmin_z);
        let tmax = tmax_x.min(tmax_y).min(tmax_z);

        // --- Условия пролета мимо ---
        // Если tmin > tmax, это значит, что луч вышел из одной плоскости (например, по X)
        // еще до того, как успел войти в другую (по Y). То есть пролетел мимо.
        // Если tmax < 0.0, значит коробка находится позади луча (сзади игрока).
        if tmin > tmax || tmax < 0.0 {
            return None;
        }

        // Если tmin < 0.0, значит tmin позади нас, а tmax впереди.
        // Это физически означает, что игрок/камера находится ПРЯМО ВНУТРИ коробки!
        if tmin < 0.0 {
            // println!("dd");
            return Some(0.0); // Пересечение прямо на старте
        }

        // Возвращаем честное расстояние до грани коробки
        Some(tmin)
    }
}

#[derive(Default)]
pub struct ECS {
    // --- Линейное движение (Linear Dynamics) ---
    pub positions: Vec<PrecisePositionC32>,
    pub velocities: Vec<Vec3>,    // Линейная скорость (м/с)
    pub accelerations: Vec<Vec3>, // Линейное ускорение (м/с²)
    pub forces: Vec<Vec3>,        // Сумма всех линейных сил (Ньютоны), сбрасывается каждый кадр

    // --- Вращательное движение (Angular Dynamics) ---
    pub orientations: Vec<Quat>, // Ориентация тела в пространстве (Кватернион)
    pub angular_velocities: Vec<Vec3>, // Угловая скорость (радианы/с, вектор задает ось и скорость)
    pub torques: Vec<Vec3>, // Сумма всех крутящих моментов (Ньютон-метры), сбрасывается каждый кадр

    // --- Масс-инерционные характеристики (Mass & Inertia) ---
    pub masses: Vec<f32>, // Масса тела (кг). Если тело статично, можно использовать f32::INFINITY
    pub inverse_masses: Vec<f32>, // 1.0 / mass. В физике чаще используют инверсию, так как на нее умножают, а не делят (ускоряет расчеты и убирает деление на 0)
    pub inverse_inertia_tensors: Vec<Mat3>, // Инвертированный тензор инерции (3х3 матрица). Описывает, как тяжело вращать тело по разным осям.

    pub object_handles: Vec<Option<PhysObjectGpuHandle>>,

    pub show_queue: Vec<usize>,
    pub hide_queue: Vec<usize>,
    pub update_queue: Vec<usize>,

    pub construct_data: Vec<PhysicsConstruction>,
}

impl ECS {
    pub fn push_default(&mut self, coords: GlobalCoords, block: Block) {
        self.positions.push(PrecisePositionC32::raw_new(
            coords.get_chunk().0,
            coords.get_local().0.as_f32() + Vector3::new(0.5, 0.5, 0.5),
        ));

        // println!("gg {}", coords.get_local().0.as_f32());

        let current_offset = self.velocities.len();

        self.velocities.push(Vec3::ZERO);
        self.accelerations.push(Vec3::ZERO);
        self.forces.push(Vec3::ZERO);
        self.orientations.push(Quat::default());
        self.angular_velocities.push(Vec3::new(0.0, 1.0, 0.0));
        self.torques.push(Vec3::ZERO);
        self.masses.push(1.0);
        self.inverse_masses.push(1.0);
        self.inverse_inertia_tensors.push(Mat3::IDENTITY);
        self.construct_data
            .push(PhysicsConstruction::one_block(block));

        self.show_queue.push(current_offset);
    }
}

pub struct PhysicsConstruction {
    pub blocks: [Block; 512],
    local_mass_center: Vec3,
    mcr_box_min: Vec3, // Mass Center Related
    mcr_box_max: Vec3,
}

impl PhysicsConstruction {
    pub fn one_block(block: Block) -> Self {
        let mut blocks = [Block::air(); 512];
        let idx = usize::from(MiniChunkCoords::new(4, 4, 4));
        blocks[idx] = block;
        PhysicsConstruction {
            blocks,
            local_mass_center: Vec3::new(4.5, 4.5, 4.5),
            mcr_box_min: Vec3::new(-0.5, -0.5, -0.5),
            mcr_box_max: Vec3::new(0.5, 0.5, 0.5),
        }
    }

    pub fn get_block(&self, coords: MiniChunkCoords) -> Block {
        self.blocks[usize::from(coords)]
    }

    pub fn get_mesh(&mut self) -> BlockIndexedPrimitive {
        let arr = into_prerender_array(&self.blocks);

        generate_mesh_generic::<PhysMiniChunk8, u8>(&arr, 0xFF)
    }

    pub fn update_box(&mut self) -> bool {
        let mut mass_sum = 0.0;
        let mut pos_sum: Vec3 = Vec3::ZERO;

        // Инициализируем min бесконечностью, а max - минус бесконечностью
        let mut box_max: Vec3 = Vec3::splat(f32::NEG_INFINITY);
        let mut box_min: Vec3 = Vec3::splat(f32::INFINITY);

        let mut has_solid_blocks = false;

        self.blocks.iter().enumerate().for_each(|(idx, block)| {
            // Обязательно проверяем, что это не воздух!
            if block.get_type().is_solid() {
                has_solid_blocks = true;

                // Масса одного блока (допустим, 1.0)
                mass_sum += 1.0;

                // Получаем чистую левую-нижнюю координату вокселя (например, от 0.0 до 7.0)
                let block_origin = Vec3::from(MiniChunkCoords::from(idx).0.as_f32());

                // Центр вокселя для расчета центра масс
                let center_pos = block_origin + Vec3::splat(0.5);
                pos_sum += center_pos;

                // Для коробки min - это левый угол блока, а max - правый дальний (+ 1.0)
                box_min = box_min.min(block_origin);
                box_max = box_max.max(block_origin + Vec3::ONE);
            }
        });

        // Если на корабле вообще нет блоков (все сломали), зануляем всё безопасными значениями
        if !has_solid_blocks {
            self.local_mass_center = Vec3::ZERO;
            self.mcr_box_max = Vec3::ZERO;
            self.mcr_box_min = Vec3::ZERO;
            return false;
        }

        // Честный центр масс
        self.local_mass_center = pos_sum / mass_sum;

        // Переносим границы коробки в пространство относительно центра масс
        self.mcr_box_max = box_max - self.local_mass_center;
        self.mcr_box_min = box_min - self.local_mass_center;
        true
    }
}
