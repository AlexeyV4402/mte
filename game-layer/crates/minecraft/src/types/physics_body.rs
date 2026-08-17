use glam::Vec3;

use crate::types::coordinates::core::{GlobalCoords, GlobalCoordsType};
use crate::types::dimension::Dimension;

pub struct PhysicsBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub size: Vec3, // Размеры хитбокса (например, 0.6, 1.8, 0.6)
    pub on_ground: bool,
}

impl PhysicsBody {
    pub fn new(position: Vec3) -> Self {
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
        // Накапливаем силы и обновляем скорость (Symplectic Euler)
        // Гравитацию (например, acceleration.y = -9.8) нужно прикладывать перед этим методом
        let move_step = self.velocity * dt;

        // --- ОСЬ X ---
        if move_step.x != 0.0 {
            self.position.x += move_step.x;
            if let Some(overlap_x) =
                Self::check_axis_collision(self.position, self.size, dimension, move_step.x, 'x')
            {
                self.position.x -= overlap_x; // Выталкиваем обратно ровно на величину захода
                self.velocity.x = 0.0; // Гасим скорость
            }
        }

        // --- ОСЬ Y (Гравитация и прыжки) ---
        if move_step.y != 0.0 {
            self.position.y += move_step.y;
            self.on_ground = false;
            if let Some(overlap_y) =
                Self::check_axis_collision(self.position, self.size, dimension, move_step.y, 'y')
            {
                self.position.y -= overlap_y;

                if self.velocity.y < 0.0 {
                    self.on_ground = true; // Упали на твердый пол
                }
                self.velocity.y = 0.0; // Обнуляем вертикальную скорость
            }
        }

        // --- ОСЬ Z ---
        if move_step.z != 0.0 {
            self.position.z += move_step.z;
            if let Some(overlap_z) =
                Self::check_axis_collision(self.position, self.size, dimension, move_step.z, 'z')
            {
                self.position.z -= overlap_z;
                self.velocity.z = 0.0;
            }
        }
    }

    fn check_axis_collision(
        pos: Vec3,
        size: Vec3,
        dimension: &Dimension,
        velocity_component: f32,
        axis: char,
    ) -> Option<f32> {
        let half_size = size * 0.5;
        let min_f = pos - half_size;
        let max_f = pos + half_size;

        // Микроскопический зазор, чтобы хитбокс не "прилипал" к грани блока
        const EPSILON: f32 = 0.0001;

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
                            if dimension
                                .get_block(GlobalCoords::from((
                                    max_x as GlobalCoordsType,
                                    y as GlobalCoordsType,
                                    z as GlobalCoordsType,
                                )))
                                .get_type()
                                .is_solid()
                            {
                                // Добавляем EPSILON, чтобы вытолкнуть чуть дальше грани
                                let overlap = max_f.x - max_x as f32 + EPSILON;
                                return Some(overlap);
                            }
                        }
                    }
                } else if velocity_component < 0.0 {
                    let min_x = min_f.x.floor() as i32;
                    for y in min_y..=max_y {
                        for z in min_z..=max_z {
                            if dimension
                                .get_block(GlobalCoords::from((
                                    min_x as GlobalCoordsType,
                                    y as GlobalCoordsType,
                                    z as GlobalCoordsType,
                                )))
                                .get_type()
                                .is_solid()
                            {
                                // Вычитаем EPSILON
                                let overlap = min_f.x - (min_x + 1) as f32 - EPSILON;
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
                            if dimension
                                .get_block(GlobalCoords::from((
                                    x as GlobalCoordsType,
                                    max_y as GlobalCoordsType,
                                    z as GlobalCoordsType,
                                )))
                                .get_type()
                                .is_solid()
                            {
                                return Some(max_f.y - max_y as f32 + EPSILON);
                            }
                        }
                    }
                } else if velocity_component < 0.0 {
                    let min_y = min_f.y.floor() as i32;
                    for x in min_x..=max_x {
                        for z in min_z..=max_z {
                            if dimension
                                .get_block(GlobalCoords::from((
                                    x as GlobalCoordsType,
                                    min_y as GlobalCoordsType,
                                    z as GlobalCoordsType,
                                )))
                                .get_type()
                                .is_solid()
                            {
                                return Some(min_f.y - (min_y + 1) as f32 - EPSILON);
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
                            if dimension
                                .get_block(GlobalCoords::from((
                                    x as GlobalCoordsType,
                                    y as GlobalCoordsType,
                                    max_z as GlobalCoordsType,
                                )))
                                .get_type()
                                .is_solid()
                            {
                                return Some(max_f.z - max_z as f32 + EPSILON);
                            }
                        }
                    }
                } else if velocity_component < 0.0 {
                    let min_z = min_f.z.floor() as i32;
                    for x in min_x..=max_x {
                        for y in min_y..=max_y {
                            if dimension
                                .get_block(GlobalCoords::from((
                                    x as GlobalCoordsType,
                                    y as GlobalCoordsType,
                                    min_z as GlobalCoordsType,
                                )))
                                .get_type()
                                .is_solid()
                            {
                                return Some(min_f.z - (min_z + 1) as f32 - EPSILON);
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
