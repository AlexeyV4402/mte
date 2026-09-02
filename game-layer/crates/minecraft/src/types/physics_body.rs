use glam::{IVec3, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;

use crate::types::coordinates::core::{ChunkCoords, GlobalCoords, GlobalCoordsType, LocalCoords};
use crate::types::dimension::Dimension;

pub struct PhysicsBody {
    pub position: PrecisePositionC32,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub size: Vec3, // Размеры хитбокса (например, 0.6, 1.8, 0.6)
    pub on_ground: bool,
}

impl PhysicsBody {
    const EPSILON: f32 = 0.0001;
    const EPSILON_VEC: Vec3 = Vec3::new(
        Self::EPSILON / 2.0,
        Self::EPSILON / 2.0,
        Self::EPSILON / 2.0,
    );

    pub fn new(position: PrecisePositionC32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            size: Vec3::new(0.6, 1.8, 0.6),
            on_ground: false,
        }
    }

    pub fn update_physics(&mut self, dt: f32) {
        // 1. Применяем трение к текущей скорости, чтобы игрок останавливался
        let friction = Vec3::new(0.85, 1.0, 0.85); // Трение по воздуху/земле
        self.velocity *= friction;

        // 2. Интегрируем ускорение в скорость (Symplectic Euler)
        // Сюда уже входят гравитация и силы от кнопок WASD
        self.velocity += self.acceleration * dt;

        // Сбрасываем ускорение для следующего кадра
        self.acceleration = Vec3::ZERO;
    }

    pub fn move_and_resolve(&mut self, dimension: &Dimension, dt: f32) {
        let move_step = self.velocity * dt;

        // --- ОСЬ X ---
        if move_step.x != 0.0 {
            self.position.in_chunk.x += move_step.x;
            if let Some(overlap_x) =
                Self::check_axis_collision(self.position, self.size, dimension, move_step.x, 'x')
            {
                self.position.in_chunk.x -= overlap_x; // Выталкиваем обратно ровно на величину захода
                self.velocity.x = 0.0; // Гасим скорость
            }
        }

        // --- ОСЬ Y (Гравитация и прыжки) ---
        if move_step.y != 0.0 {
            self.position.in_chunk.y += move_step.y;
            self.on_ground = false;
            if let Some(overlap_y) =
                Self::check_axis_collision(self.position, self.size, dimension, move_step.y, 'y')
            {
                self.position.in_chunk.y -= overlap_y;

                if self.velocity.y < 0.0 {
                    self.on_ground = true; // Упали на твердый пол
                }
                self.velocity.y = 0.0; // Обнуляем вертикальную скорость
            }
        }

        // --- ОСЬ Z ---
        if move_step.z != 0.0 {
            self.position.in_chunk.z += move_step.z;
            if let Some(overlap_z) =
                Self::check_axis_collision(self.position, self.size, dimension, move_step.z, 'z')
            {
                self.position.in_chunk.z -= overlap_z;
                self.velocity.z = 0.0;
            }
        }

        self.position.normalize();
    }

    pub fn overlap_with(&self, coords: GlobalCoords) -> bool {
        let block_position = coords.0;
        let pos: Vec3 = self.position.in_chunk.into();
        let half_size = self.size * 0.5;
        let min_f = pos - half_size - Self::EPSILON_VEC;
        let max_f = pos + half_size + Self::EPSILON_VEC;

        let local_min: Vector3<i32> = min_f.floor().as_ivec3().into();
        let local_max: Vector3<i32> = max_f.floor().as_ivec3().into();

        let chunk_shift_i64 = self.position.chunk.as_i64().shl_all(5);

        let global_min = chunk_shift_i64 + local_min.as_i64();
        let global_max = chunk_shift_i64 + local_max.as_i64();

        block_position.all_ge_cw(global_min) && block_position.all_le_cw(global_max)
    }

    fn check_axis_collision(
        pos: PrecisePositionC32,
        size: Vec3,
        dimension: &Dimension,
        velocity_component: f32,
        axis: char,
    ) -> Option<f32> {
        let central_chunk = ChunkCoords::from(pos.chunk);

        let pos: Vec3 = pos.in_chunk.into();

        let half_size = size * 0.5;
        let min_f = pos - half_size;
        let max_f = pos + half_size;

        match axis {
            'x' => {
                let min_y = min_f.y.floor() as i32;
                let max_y = max_f.y.floor() as i32;
                let min_z = min_f.z.floor() as i32;
                let max_z = max_f.z.floor() as i32;

                if velocity_component > 0.0 {
                    let max_x = max_f.x.floor() as i32;
                    for y in min_y..=max_y {
                        for z in min_z..=max_z {
                            let block_coords = GlobalCoords::from((
                                central_chunk,
                                LocalCoords::new(max_x as u32, y as u32, z as u32),
                            ));
                            if dimension
                                .get_block_loaded(block_coords)
                                .get_type()
                                .is_solid()
                            {
                                let overlap = max_f.x - max_x as f32 + Self::EPSILON;
                                return Some(overlap);
                            }
                        }
                    }
                } else if velocity_component < 0.0 {
                    let min_x = min_f.x.floor() as i32;
                    for y in min_y..=max_y {
                        for z in min_z..=max_z {
                            let block_coords = GlobalCoords::from((
                                central_chunk,
                                LocalCoords::new(min_x as u32, y as u32, z as u32),
                            ));
                            if dimension
                                .get_block_loaded(block_coords)
                                .get_type()
                                .is_solid()
                            {
                                let overlap = min_f.x - (min_x + 1) as f32 - Self::EPSILON;
                                return Some(overlap);
                            }
                        }
                    }
                }
            }
            'y' => {
                let min_x = min_f.x.floor() as i32;
                let max_x = max_f.x.floor() as i32;
                let min_z = min_f.z.floor() as i32;
                let max_z = max_f.z.floor() as i32;

                if velocity_component > 0.0 {
                    let max_y = max_f.y.floor() as i32;
                    for x in min_x..=max_x {
                        for z in min_z..=max_z {
                            let block_coords = GlobalCoords::from((
                                central_chunk,
                                LocalCoords::new(x as u32, max_y as u32, z as u32),
                            ));
                            if dimension
                                .get_block_loaded(block_coords)
                                .get_type()
                                .is_solid()
                            {
                                return Some(max_f.y - max_y as f32 + Self::EPSILON);
                            }
                        }
                    }
                } else if velocity_component < 0.0 {
                    let min_y = min_f.y.floor() as i32;
                    for x in min_x..=max_x {
                        for z in min_z..=max_z {
                            let block_coords = GlobalCoords::from((
                                central_chunk,
                                LocalCoords::new(x as u32, min_y as u32, z as u32),
                            ));
                            if dimension
                                .get_block_loaded(block_coords)
                                .get_type()
                                .is_solid()
                            {
                                return Some(min_f.y - (min_y + 1) as f32 - Self::EPSILON);
                            }
                        }
                    }
                }
            }
            'z' => {
                let min_x = min_f.x.floor() as i32;
                let max_x = max_f.x.floor() as i32;
                let min_y = min_f.y.floor() as i32;
                let max_y = max_f.y.floor() as i32;

                if velocity_component > 0.0 {
                    let max_z = max_f.z.floor() as i32;
                    for x in min_x..=max_x {
                        for y in min_y..=max_y {
                            let block_coords = GlobalCoords::from((
                                central_chunk,
                                LocalCoords::new(x as u32, y as u32, max_z as u32),
                            ));
                            if dimension
                                .get_block_loaded(block_coords)
                                .get_type()
                                .is_solid()
                            {
                                return Some(max_f.z - max_z as f32 + Self::EPSILON);
                            }
                        }
                    }
                } else if velocity_component < 0.0 {
                    let min_z = min_f.z.floor() as i32;
                    for x in min_x..=max_x {
                        for y in min_y..=max_y {
                            let block_coords = GlobalCoords::from((
                                central_chunk,
                                LocalCoords::new(x as u32, y as u32, min_z as u32),
                            ));
                            if dimension
                                .get_block_loaded(block_coords)
                                .get_type()
                                .is_solid()
                            {
                                return Some(min_f.z - (min_z + 1) as f32 - Self::EPSILON);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    pub fn on_ground(&self) -> bool {
        self.on_ground
    }
}
