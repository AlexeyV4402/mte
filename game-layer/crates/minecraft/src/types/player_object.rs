use std::mem::transmute;
use std::time::{Duration, Instant};

use glam::{Mat4, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;
use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::render_objects::camera::{
    RotatableCamera, WorldCameraUniform
};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::raycast::raycast;
use crate::types;
use crate::types::blocks::block::{Block, BlockType, REGISTERED_BLOCKS_COUNT};
use crate::types::dimension::Dimension;
use crate::types::item::{Item, ItemType};
use crate::types::physics_body::PhysicsBody;

pub struct PlayerObject {
    physic_body: PhysicsBody,
    hand_item: Item,
    pub camera: RotatableCamera,
    hand_dirty: bool,
}

const MOVE_FORCE: f32 = 200.0;
const GRAVITY_FORCE: Vec3 = Vec3::new(0.0, -35.0, 0.0);
const JUMP_SPEED: f32 = 12.0;

impl PlayerObject {
    const CAMERA_OFFSET: PrecisePositionC32 = PrecisePositionC32 {
        chunk: Vector3::<i32>::new(0, 0, 0),
        in_chunk: Vector3::<f32>::new(0.0, 0.7, 0.0),
    };

    pub fn new(position: PrecisePositionC32, camera: RotatableCamera) -> Self {
        Self {
            physic_body: PhysicsBody::new(position),
            hand_item: Item::from_block(
                Block::from_type(crate::types::blocks::block::BlockType::Dirt),
                1,
            ),
            camera,
            hand_dirty: true,
        }
    }

    pub fn update(&mut self, input_state: &InputState, dt: Duration, dimension: &mut Dimension) {
        self.camera.update(input_state, dt);
        let dt = dt.as_secs_f32();
        self.physic_body.update_physics(dt);

        let forward = self.camera.lens.get_direction();

        let right = Vec3::Y.cross(forward).normalize_or_zero();

        let get_axis = |key: KeyCode| input_state.is_down(key) as u32 as f32;

        let move_forward = get_axis(KeyCode::KeyW) - get_axis(KeyCode::KeyS);
        let move_right = get_axis(KeyCode::KeyD) - get_axis(KeyCode::KeyA);

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

        let origin = self.physic_body.position.raw_add(Self::CAMERA_OFFSET);
        let direction = forward;

        // let start = Instant::now();
        let raycast_result = raycast(dimension, origin, direction, 8.0);
        // println!("Поиск рейкаста: {} ms", start.elapsed().as_millis());

        if let Some(raycast) = raycast_result {
            if input_state.is_mouse_just_pressed(MouseButton::Left) {
                dimension.set_block_loaded(raycast.target_block, Block::default());
            }
            if let types::item::ItemType::Block(block) = self.get_hand_item().item_type {
                if input_state.is_mouse_just_pressed(MouseButton::Right) {
                    if !self.physic_body.overlap_with(raycast.previous_block) {
                        dimension.set_block_loaded(raycast.previous_block, block);
                    }
                }
            }
        }

        // Теперь двигаем тело и разрешаем коллизии
        self.physic_body.move_and_resolve(dimension, dt);
    }

    #[inline]
    pub fn get_hand_item(&self) -> Item {
        self.hand_item
    }

    pub fn get_position(&self) -> PrecisePositionC32 {
        self.physic_body.position
    }

    pub fn get_camera_position(&self) -> PrecisePositionC32 {
        self.physic_body
            .position
            .raw_add(Self::CAMERA_OFFSET)
            .normalized()
    }

    pub fn get_camera_world_uniform(&self) -> WorldCameraUniform {
        self.camera
            .lens
            .get_world_uniform(self.get_camera_position())
    }

    pub fn get_camera_hand_uniform(&self) -> [[f32; 4]; 4] {
        self.camera.lens.get_proj_mat().to_cols_array_2d()
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

            renderer.buffer_manager.write_with_matrix(
                0,
                &self.hand_item.item_type.get_hand_model(),
                &final_hand_model.to_cols_array_2d(),
            );
            self.hand_dirty = false;
        }
    }
}
