use std::mem::transmute;
use std::time::Duration;

use glam::{Mat4, Vec3};
use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::Renderer;
use winit::keyboard::KeyCode;

use crate::types::blocks::block::{Block, BlockType, REGISTERED_BLOCKS_COUNT};
use crate::types::dimension::Dimension;
use crate::types::item::{Item, ItemType};
use crate::types::physics_body::PhysicsBody;

pub struct PlayerObject {
    physic_body: PhysicsBody,
    hand_item: Item,
    hand_dirty: bool,
}

const MOVE_FORCE: f32 = 200.0;
const GRAVITY_FORCE: Vec3 = Vec3::new(0.0, -35.0, 0.0);
const JUMP_SPEED: f32 = 12.0;

impl PlayerObject {
    pub fn new(position: Vec3) -> Self {
        Self {
            physic_body: PhysicsBody::new(position),
            hand_item: Item::from_block(
                Block::from_type(crate::types::blocks::block::BlockType::Dirt),
                1,
            ),
            hand_dirty: true,
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
        let move_up =
            (input_state.is_down(KeyCode::Space) && self.physic_body.on_ground()) as u32 as f32;

        let move_force = get_axis(KeyCode::ShiftLeft) * MOVE_FORCE + MOVE_FORCE;

        let acceleration = forward.with_y(0.0).normalize_or_zero() * move_forward * move_force
            + right * move_right * move_force
            + GRAVITY_FORCE;

        self.physic_body.acceleration = acceleration;
        self.physic_body.velocity.y += JUMP_SPEED * move_up;

        let mouse_wheel_delta = input_state.mouse_scroll_delta as i16;

        match &mut self.hand_item.item_type {
            ItemType::Block(block) => {
                self.hand_dirty = mouse_wheel_delta != 0;
                println!("{}", self.hand_dirty);
                *block = block.with_type(unsafe {
                    transmute::<u16, BlockType>(
                        (transmute::<BlockType, u16>(block.get_type()) as i16 + mouse_wheel_delta)
                            .clamp(1, REGISTERED_BLOCKS_COUNT as i16 - 1)
                            as u16,
                    )
                })
            }
            ItemType::Item(true_item) => todo!(),
        }

        // Теперь двигаем тело и разрешаем коллизии
        self.physic_body.move_and_resolve(dimension, dt);
    }

    #[inline]
    pub fn get_hand_item(&self) -> Item {
        self.hand_item
    }

    pub fn get_position(&self) -> Vec3 {
        self.physic_body.position
    }

    pub fn update_inventory_meshes(&mut self, renderer: &mut Renderer) {
        if self.hand_dirty {
            // renderer.buffer_manager.unload_mesh(0);
            let hand_offset = Vec3::new(0.2, -0.4, 1.5);

            // Матрица модели для руки — это просто сдвиг на этот вектор!
            let hand_model_matrix = Mat4::from_translation(hand_offset);

            // Если ты захочешь, чтобы блок был чуть-чуть повернут к игроку красивым боком (как в Майнкрафте):
            let hand_rotation = Mat4::from_rotation_y(45.0f32.to_radians())
                * Mat4::from_rotation_x(30.0f32.to_radians());

            // Итоговая матрица модели для руки (Сдвиг * Поворот):
            let final_hand_model = hand_rotation * hand_model_matrix;

            // println!("Матрица модели: {}", final_hand_model);

            renderer.buffer_manager.write_to_slot(
                0,
                &self.hand_item.item_type.get_hand_model(),
                &final_hand_model.to_cols_array_2d(),
            );
            self.hand_dirty = false;
        }
    }
}
