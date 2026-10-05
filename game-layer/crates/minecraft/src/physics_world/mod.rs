pub mod components;
pub mod modules;
pub mod physics_construction;
pub mod utils;

use std::time::Duration;

use glam::{Mat3, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::game_renderer::VkBackend;

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

// const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

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

        let update_result = match self.data.construct_data[idx].update() {
            Some(result) => result,
            None => {
                return;
            }
        };

        self.data.masses[idx] = update_result.mass;
        self.data.inverse_inertia_tensors[idx] = update_result.inverse_inertia;

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

        self.system_apply_damping(delta_time);

        self.integrate_module
            .system_integrate_orientations(&mut self.data, delta_time);
        self.integrate_module
            .system_integrate_positions(&mut self.data, delta_time);

        self.integrate_module.system_clear_forces(&mut self.data);

        self.system_resolve_collisions(overworld, delta_time);
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

    #[inline(always)]
    fn system_apply_damping(&mut self, delta_time: f32) {
        // Коэффициенты затухания (подбираются на глаз, ~0.98 - 0.99)
        let linear_damping = f32::powf(0.98, delta_time);
        let angular_damping = f32::powf(0.95, delta_time); // Угловое гасим чуть сильнее, чтобы убрать раскачку

        for i in 0..self.data.velocities.len() {
            if self.data.inverse_masses[i] > 0.0 {
                self.data.velocities[i] *= linear_damping;
                self.data.angular_velocities[i] *= angular_damping;
            }
        }
    }

    pub fn test_voxel_vs_voxel(
        &self,
        static_block_pos: Vec3, // Координаты угла блока земли (целые числа в f32)
        flying_voxel_center: Vec3,
    ) -> Option<ContactPoint> {
        // 1. Границы блока земли (AABB)
        let b_min = static_block_pos;
        let b_max = static_block_pos + Vec3::ONE;

        // 2. Границы летящего блока корабля (AABB)
        let a_min = flying_voxel_center - Vec3::splat(0.5);
        let a_max = flying_voxel_center + Vec3::splat(0.5);

        // 3. Честная проверка на пересечение AABB по всем осям
        if a_max.x <= b_min.x
            || a_min.x >= b_max.x
            || a_max.y <= b_min.y
            || a_min.y >= b_max.y
            || a_max.z <= b_min.z
            || a_min.z >= b_max.z
        {
            return None; // Не пересекаются
        }

        // 4. Вычисляем глубину проникновения с каждой из 6 сторон
        let overlap_right = a_max.x - b_min.x; // пробил левую грань земли
        let overlap_left = b_max.x - a_min.x; // пробил правую грань земли
        let overlap_top = a_max.y - b_min.y; // пробил нижнюю грань земли
        let overlap_bottom = b_max.y - a_min.y; // пробил верхнюю грань земли (пол)
        let overlap_front = a_max.z - b_min.z; // пробил ближнюю грань земли
        let overlap_back = b_max.z - a_min.z; // пробил дальнюю грань земли

        // 5. Ищем ось с минимальным перекрытием
        let mut normal = Vec3::ZERO;
        let mut penetration = f32::INFINITY;

        // Ось X
        let min_x = overlap_right.min(overlap_left);
        if min_x < penetration {
            penetration = min_x;
            normal = if overlap_right < overlap_left {
                Vec3::X
            } else {
                -Vec3::X
            };
        }

        // Ось Y
        let min_y = overlap_top.min(overlap_bottom);
        if min_y < penetration {
            penetration = min_y;
            normal = if overlap_top < overlap_bottom {
                Vec3::Y
            } else {
                -Vec3::Y
            };
        }

        // Ось Z (Исправлено: теперь проверяем именно Z, а не дублируем Y)
        let min_z = overlap_front.min(overlap_back);
        if min_z < penetration {
            penetration = min_z;
            normal = if overlap_front < overlap_back {
                Vec3::Z
            } else {
                -Vec3::Z
            };
        }

        // Точка контакта на поверхности соударения
        let contact_pos = flying_voxel_center - (normal * (0.5 - penetration * 0.5));

        // Важная корректировка: нормаль должна смотреть ИЗ земли НАРУЖУ (навстречу летящему блоку).
        // Если объект падает сверху (-Y) на пол, то нормаль должна быть строго ВВЕРХ (+Y).
        // Инвертируем нормаль, чтобы она стала выталкивающей для решателя импульсов:
        let outward_normal = -normal;

        Some(ContactPoint {
            position: contact_pos,
            normal: outward_normal,
            penetration,
        })
    }

    pub fn resolve_contacts(&mut self, idx: usize, contacts: &[ContactPoint], delta_time: f32) {
        if contacts.is_empty() || self.data.inverse_masses[idx] == 0.0 {
            return;
        }

        let inv_mass = self.data.inverse_masses[idx];
        let inv_inertia_local = self.data.inverse_inertia_tensors[idx];
        let rotation = self.data.orientations[idx];

        let rot_mat = Mat3::from_quat(rotation);
        let inv_inertia_world = rot_mat * inv_inertia_local * rot_mat.transpose();

        let restitution = 0.0; // Для балансирующих конструкций отскок лучше занулить (0.0 - неупругий удар)

        // --- КОНСТАНТЫ СТАБИЛИЗАЦИИ БАУМГАРТЕ ---
        const PENETRATION_SLOP: f32 = 0.005; // 5 мм зона покоя. Меньше этого - коллизия "спит"
        const BAUMGARTE_FACTOR: f32 = 0.2; // Выталкиваем только на 20% за кадр

        for contact in contacts {
            // Если проникновение слишком мелкое — игнорируем, чтобы убрать микро-дрожание
            if contact.penetration < PENETRATION_SLOP {
                continue;
            }

            // 1. ИСПРАВЛЕННОЕ ВЫТАЛКИВАНИЕ (Баумгарте)
            // Вместо жесткой телепортации сдвигаем плавно
            if contact.penetration > PENETRATION_SLOP {
                // Выталкиваем геометрически, но аккуратно
                let fill_depth = (contact.penetration - PENETRATION_SLOP) * BAUMGARTE_FACTOR;
                self.data.positions[idx].in_chunk.x += contact.normal.x * fill_depth;
                self.data.positions[idx].in_chunk.y += contact.normal.y * fill_depth;
                self.data.positions[idx].in_chunk.z += contact.normal.z * fill_depth;
                self.data.positions[idx].normalize();
            }

            // 2. СКОРОСТЬ ТОЧКИ КОНТАКТА
            let ship_world_pos: Vec3 = self.data.positions[idx].in_chunk.into();
            let r = contact.position - ship_world_pos;
            let v_contact = self.data.velocities[idx] + self.data.angular_velocities[idx].cross(r);
            let vel_along_normal = v_contact.dot(contact.normal);

            // Если точка уже удаляется от земли, импульс НЕ прикладываем
            if vel_along_normal > 0.0 {
                continue;
            }

            // 3. РАСЧЕТ ПОЗИЦИОННОЙ ДОБАВКИ (BIAS)
            // Защищаем bias: выталкивающая скорость не должна быть слишком огромной.
            // Ограничим её константой (например, максимум 2.0 м/с), чтобы объект не взлетал как ракета.
            let raw_bias =
                (BAUMGARTE_FACTOR / delta_time) * (contact.penetration - PENETRATION_SLOP).max(0.0);
            let position_bias = raw_bias.min(5.0); // МАКСИМАЛЬНЫЙ ЛИМИТ ВЫСТРЕДА ВВЕРХ

            // 4. ЦЕЛЕВАЯ СКОРОСТЬ ОТСКОКА
            // Выставляем restitution строго в 0.0! Нам не нужен батут, нам нужно гасить удар.
            let restitution = 0.0;
            let target_vel = -vel_along_normal * (1.0 + restitution) + position_bias;
            let inertia_torque = (inv_inertia_world * r.cross(contact.normal)).cross(r);
            let angular_factor = inertia_torque.dot(contact.normal);
            // 5. РАСЧЕТ И ПРИМЕНЕНИЕ ИМПУЛЬСА БАРАФФА
            let denominator = inv_mass + angular_factor;
            if denominator == 0.0 {
                continue;
            }

            let j = target_vel / denominator;
            let impulse = contact.normal * j;

            // Мгновенно гасим скорости в ECS
            self.data.velocities[idx] += impulse * inv_mass;
            self.data.angular_velocities[idx] += inv_inertia_world * r.cross(impulse);
        }
    }

    pub fn system_resolve_collisions(&mut self, overworld: &Dimension, delta_time: f32) {
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

                        let v_min = voxel_world_center - Vec3::splat(0.5);
                        let v_max = voxel_world_center + Vec3::splat(0.5);

                        // Находим целочисленные границы блоков в локальном пространстве чанка корабля
                        let start_x = v_min.x.floor() as i32;
                        let end_x = v_max.x.floor() as i32;
                        let start_y = v_min.y.floor() as i32;
                        let end_y = v_max.y.floor() as i32;
                        let start_z = v_min.z.floor() as i32;
                        let end_z = v_max.z.floor() as i32;

                        // Крутим микро-циклы по локальным индексам блоков
                        for bx in start_x..=end_x {
                            for by in start_y..=end_y {
                                for bz in start_z..=end_z {
                                    // 2. Собираем PrecisePosition для проверки блока в overworld.
                                    // Мы стартуем ОТ чанка корабля и прибавляем локальные f32 координаты!
                                    let mut block_precise_pos = PrecisePositionC32 {
                                        chunk: self.data.positions[idx].chunk,
                                        in_chunk: Vec3::new(
                                            bx as f32 + 0.5,
                                            by as f32 + 0.5,
                                            bz as f32 + 0.5,
                                        )
                                        .into(),
                                    };

                                    // Вот теперь нормализация сделает свою работу ЧЕСТНО:
                                    // она сама раскидает координаты по чанкам, если блок земли вышел за границы чанка корабля
                                    block_precise_pos.normalize();

                                    // Опрашиваем мир по честным глобальным координатам блока
                                    let world_block = overworld
                                        .get_block_loaded(block_precise_pos.get_global_coords());

                                    if world_block.get_type().is_solid() {
                                        // 3. ПЕРЕВОДИМ БЛОК ЗЕМЛИ В ПРОСТРАНСТВО КОРАБЛЯ (ЧЕСТНАЯ МАТЕМАТИКА):
                                        // Теперь, когда block_precise_pos нормализован, мы берем ЕГО актуальный in_chunk
                                        // и честный chunk_diff. Никаких старых ненормализованных `bx`!
                                        let chunk_diff = block_precise_pos.chunk
                                            - self.data.positions[idx].chunk;

                                        let static_block_pos = chunk_diff.as_f32() * 32.0
                                            + Vec3::new(
                                                block_precise_pos.in_chunk.x.floor(),
                                                block_precise_pos.in_chunk.y.floor(),
                                                block_precise_pos.in_chunk.z.floor(),
                                            )
                                            .into();

                                        // 4. Запускаем тест вокселей. Координаты теперь ювелирно точны!
                                        if let Some(contact) = self.test_voxel_vs_voxel(
                                            static_block_pos.into(),
                                            voxel_world_center,
                                        ) {
                                            contacts.push(contact);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // contacts.iter().for_each(|f| {
            //     if f.penetration > 0.1 {
            //         println!("penetration: {}", f.penetration);
            //     }

            // });

            // Передаем собранные контакты в импульсный решатель скоростей
            self.resolve_contacts(idx, &contacts, delta_time);
        }
    }
}

pub struct ContactPoint {
    pub position: Vec3,   // Точка удара в f32 пространстве мира
    pub normal: Vec3,     // Нормаль грани земли (куда выталкивать)
    pub penetration: f32, // Глубина проникновения (метров)
}
