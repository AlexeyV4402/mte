pub mod components;
pub mod modules;
pub mod physics_construction;
pub mod utils;

use std::time::Duration;

use glam::{Mat3, Vec3};
use lib_core::math::vectors::custom::{PrecisePosition, PrecisePositionC32};
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;

use crate::physics_world::components::Components;
use crate::physics_world::modules::integrate::IntegrateModule;
use crate::physics_world::modules::raycast::RaycastModule;
use crate::physics_world::modules::render::RenderModule;
use crate::types::blocks::block::Block;
use crate::types::coordinates::core::{MiniChunkCoords, PrecisePositionC32Coords};
use crate::types::dimension::Dimension;
use crate::types::world::PhysicsEvent;
use crate::utils::raycast::RaycastResult;

#[derive(Default)]
pub struct PhysicsWorld {
    pub data: Components,
    pub render_module: RenderModule,
    pub integrate_module: IntegrateModule,
    pub raycast_module: RaycastModule,
}

const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

impl PhysicsWorld {
    pub fn read_queue(&mut self, queue: &mut Vec<PhysicsEvent>) {
        queue.drain(..).for_each(|event| match event {
            PhysicsEvent::PlaceBlock(coords, block) => {
                let current_offset = self.data.push_default(coords, block);
                self.render_module.show_queue.push(current_offset);
            }
        });
    }

    pub fn set_block(&mut self, idx: usize, coords: MiniChunkCoords, block: Block) {
        self.data.construct_data[idx].blocks[usize::from(coords)] = block;
        let old_mass_center = self.data.construct_data[idx].local_mass_center;

        let exists = self.data.construct_data[idx].update_box();

        let new_mass_center = self.data.construct_data[idx].local_mass_center;
        self.data.positions[idx].in_chunk += (new_mass_center - old_mass_center).into();
        self.render_module.show_queue.push(idx);
    }

    pub fn update_meshes(&mut self, renderer: &mut VkBackend) {
        self.render_module.update_meshes(renderer, &self.data);
    }

    pub fn update(&mut self, delta_time: Duration, overworld: &Dimension) {
        let delta_time = delta_time.as_secs_f32();

        self.integrate_module.system_apply_gravity(&mut self.data);

        // self.system_apply_grav_gun(player_target);
        // self.system_apply_aerodynamics();

        self.integrate_module.system_integrate_velocities(
            &mut self.data,
            &mut self.render_module,
            delta_time,
        );

        self.integrate_module
            .system_integrate_orientations(&mut self.data, delta_time);
        self.integrate_module
            .system_integrate_positions(&mut self.data, delta_time);

        self.integrate_module.system_clear_forces(&mut self.data);

        self.system_resolve_collisions(overworld);
    }

    pub fn raycast_physics(
        &self,
        origin: PrecisePositionC32,
        dir: Vec3,
        max_dist: f32,
    ) -> Option<(RaycastResult<MiniChunkCoords>, usize)> {
        self.raycast_module.raycast_physics(
            &self.data.construct_data,
            &self.data.positions,
            &self.data.orientations,
            origin,
            dir,
            max_dist,
        )
    }

    pub fn test_voxel_vs_voxel(
        &self,
        static_block_pos: Vec3,
        flying_voxel_center: Vec3,
    ) -> Option<ContactPoint> {
        // Границы блока земли (AABB)
        let b_min = static_block_pos;
        let b_max = static_block_pos + Vec3::ONE;

        // Границы летящего блока корабля (AABB)
        let a_min = flying_voxel_center - Vec3::splat(0.5);
        let a_max = flying_voxel_center + Vec3::splat(0.5);

        // Проверяем, есть ли вообще пересечение по всем трем осям
        if a_max.x <= b_min.x
            || a_min.x >= b_max.x
            || a_max.y <= b_min.y
            || a_min.y >= b_max.y
            || a_max.z <= b_min.z
            || a_min.z >= b_max.z
        {
            return None; // Мимо
        }

        // Вычисляем глубину проникновения по каждой оси отдельно
        let overlap_x = (a_max.x - b_min.x).min(b_max.x - a_min.x);
        let overlap_y = (a_max.y - b_min.y).min(b_max.y - a_min.y);
        let overlap_z = (a_max.z - b_min.z).min(b_max.z - a_min.z);

        // Находим ось с НАИМЕНЬШИМ проникновением — это и есть плоскость удара!
        let (mut normal, penetration) = if overlap_x < overlap_y && overlap_x < overlap_z {
            // Удар пришелся в боковую грань X
            let sign = if flying_voxel_center.x > static_block_pos.x + 0.5 {
                1.0
            } else {
                -1.0
            };
            (Vec3::new(sign, 0.0, 0.0), overlap_x)
        } else if overlap_y < overlap_z {
            // Удар пришелся в пол/потолок Y
            let sign = if flying_voxel_center.y > static_block_pos.y + 0.5 {
                1.0
            } else {
                -1.0
            };
            (Vec3::new(0.0, sign, 0.0), overlap_y)
        } else {
            // Удар пришелся в переднюю/заднюю грань Z
            let sign = if flying_voxel_center.z > static_block_pos.z + 0.5 {
                1.0
            } else {
                -1.0
            };
            (Vec3::new(0.0, 0.0, sign), overlap_z)
        };

        // Точка контакта — это центр пересечения (приблизительно)
        let contact_pos = flying_voxel_center - (normal * (0.5 - penetration * 0.5));

        Some(ContactPoint {
            position: contact_pos,
            normal,
            penetration,
        })
    }

    pub fn resolve_contacts(&mut self, idx: usize, contacts: &[ContactPoint]) {
        if contacts.is_empty() || self.data.inverse_masses[idx] == 0.0 {
            return;
        }

        let inv_mass = self.data.inverse_masses[idx];
        let inv_inertia_local = self.data.inverse_inertia_tensors[idx];
        let rotation = self.data.orientations[idx];

        // Переводим тензор инерции в мировые координаты с учетом текущего кватерниона
        let rot_mat = Mat3::from_quat(rotation);
        let inv_inertia_world = rot_mat * inv_inertia_local * rot_mat.transpose();

        let restitution = 0.0; // Коэффициент отскока (0.0 - липнет, 1.0 - прыгает как резина)

        for contact in contacts {
            // 1. ГЕОМЕТРИЧЕСКОЕ ВЫТАЛКИВАНИЕ (Чтобы блоки не застревали)
            // Распределяем выталкивание: сдвигаем PrecisePosition
            self.data.positions[idx].in_chunk += (contact.normal * contact.penetration).into();
            self.data.positions[idx].normalize();

            // 2. ВЕКТОР РЫЧАГА (От физического центра масс до точки удара)
            let ship_world_pos: Vec3 = self.data.positions[idx].in_chunk.into();
            let r = contact.position - ship_world_pos;

            // 3. ОТНОСИТЕЛЬНАЯ СКОРОСТЬ ТОЧКИ КОНТАКТА
            // Скорость конкретного вокселя с учетом вращения корабля!
            let v_contact = self.data.velocities[idx] + self.data.angular_velocities[idx].cross(r);
            let vel_along_normal = v_contact.dot(contact.normal);

            // Если воксель уже движется ОТ земли (отскакивает), импульс не нужен
            if vel_along_normal > 0.0 {
                continue;
            }

            // 4. ВЛИЯНИЕ ВРАЩЕНИЯ НА УДАР (Знаменатель инерции)
            let inertia_torque = (inv_inertia_world * r.cross(contact.normal)).cross(r);
            let angular_factor = inertia_torque.dot(contact.normal);

            // 5. РАСЧЕТ ИМПУЛЬСА (Магия Бараффа)
            let denominator = inv_mass + angular_factor;
            if denominator == 0.0 {
                continue;
            }

            let j = -(1.0 + restitution) * vel_along_normal / denominator;
            let impulse = contact.normal * j;

            // 6. МГНОВЕННОЕ ОБНОВЛЕНИЕ СКОРОСТЕЙ В ECS
            // Изменяем линейную скорость
            self.data.velocities[idx] += impulse * inv_mass;

            // Изменяем угловую скорость (заставляем крутиться от удара углом!)
            self.data.angular_velocities[idx] += inv_inertia_world * r.cross(impulse);
        }
    }

    pub fn system_resolve_collisions(&mut self, overworld: &Dimension) {
        let len = self.data.positions.len();

        for idx in 0..len {
            if self.data.inverse_masses[idx] == 0.0 {
                continue;
            }

            let mut contacts = Vec::new();
            let ship_world_pos: Vec3 = self.data.positions[idx].in_chunk.into();
            let rot_mat = Mat3::from_quat(self.data.orientations[idx]);
            let local_mass_center = self.data.construct_data[idx].local_mass_center;

            // Перебираем все воксели внутри твоего массива 8х8х8
            for x in 0..8 {
                for y in 0..8 {
                    for z in 0..8 {
                        let block =
                            self.data.construct_data[idx].get_block(MiniChunkCoords::new(x, y, z));
                        if !block.get_type().is_solid() {
                            continue;
                        }

                        // Находим f32 позицию центра этого вокселя в мире с учетом ПОВОРОТА корабля
                        let local_pos = Vec3::new(x as f32, y as f32, z as f32) + Vec3::splat(0.5);
                        let mcr_pos = local_pos - local_mass_center; // Относительно центра масс
                        let voxel_world_center = rot_mat * mcr_pos + ship_world_pos;

                        // Переводим эту f32 точку в PrecisePosition, чтобы опросить чанки мира
                        let mut voxel_precise_pos = PrecisePositionC32 {
                            chunk: self.data.positions[idx].chunk,
                            in_chunk: voxel_world_center.into(),
                        };
                        voxel_precise_pos.normalize();

                        // Ищем, есть ли в этой точке мира твердый блок земли
                        let world_block =
                            overworld.get_block_loaded(voxel_precise_pos.get_global_coords());
                        if world_block.get_type().is_solid() {
                            // Блок земли найден! Вычисляем f32-координаты этого блока земли в мире
                            // (Для простоты примера переводим чанк земли в f32 относительно чанка корабля)
                            let chunk_diff =
                                voxel_precise_pos.chunk - self.data.positions[idx].chunk;
                            let static_block_pos =
                                chunk_diff.as_f32() * 32.0 + voxel_precise_pos.in_chunk.floor_cw();

                            // Запускаем геометрический тест вокселей
                            if let Some(contact) = self
                                .test_voxel_vs_voxel(static_block_pos.into(), voxel_world_center)
                            {
                                contacts.push(contact);
                            }
                        }
                    }
                }
            }

            // Передаем собранные контакты в импульсный решатель скоростей
            self.resolve_contacts(idx, &contacts);
        }
    }
}

pub struct ContactPoint {
    pub position: Vec3,   // Точка удара в f32 пространстве мира
    pub normal: Vec3,     // Нормаль грани земли (куда выталкивать)
    pub penetration: f32, // Глубина проникновения (метров)
}
