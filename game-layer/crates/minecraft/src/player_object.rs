use std::time::Duration;

use glam::{Vec2, Vec3, Vec3Swizzles};
use lib_io::user_io::InputState;
use winit::keyboard::KeyCode;

use crate::dimension::Dimension;
use crate::physics_body::PhysicsBody;
use crate::types::coordinates::core::{GlobalCoords, GlobalCoordsType};

pub struct PlayerObject {
    physic_body: PhysicsBody,
}

const MOVE_FORCE: f32 = 200.0;
const GRAVITY_FORCE: Vec3 = Vec3::new(0.0, -35.0, 0.0);
const JUMP_SPEED: f32 = 12.0;

impl PlayerObject {
    pub fn new(position: Vec3) -> Self {
        Self {
            physic_body: PhysicsBody::new(position),
        }
    }

    pub fn update(
        &mut self,
        input_state: &InputState,
        dt: Duration,
        forward: Vec3,
        dimension: &Dimension,
    ) {
        let dt = dt.as_secs_f32();
        self.physic_body.update_physics(dt);

        let right = Vec3::Y.cross(forward).normalize_or_zero();

        let get_axis = |key: KeyCode| input_state.is_down(key) as u32 as f32;

        let move_forward = get_axis(KeyCode::KeyW) - get_axis(KeyCode::KeyS);
        let move_right = get_axis(KeyCode::KeyD) - get_axis(KeyCode::KeyA);

        // let move_up = get_axis(KeyCode::Space) - get_axis(KeyCode::ControlLeft);
        let move_up = (input_state.is_down(KeyCode::Space) && self.physic_body.on_ground()) as u32 as f32;

        let move_force = get_axis(KeyCode::ShiftLeft) * MOVE_FORCE + MOVE_FORCE;

        let acceleration = forward.with_y(0.0).normalize_or_zero() * move_forward * move_force
            + right * move_right * move_force
            + GRAVITY_FORCE;

        self.physic_body.acceleration = acceleration;
        self.physic_body.velocity.y += JUMP_SPEED * move_up;

        // Теперь двигаем тело и разрешаем коллизии
        self.physic_body.move_and_resolve(dimension, dt);
    }

    pub fn get_position(&self) -> Vec3 {
        self.physic_body.position
    }
}
