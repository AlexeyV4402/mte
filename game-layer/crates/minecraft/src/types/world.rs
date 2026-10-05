use std::time::Duration;

use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;
use lib_core::math::vectors::vec3::types::{Vec3f32, Vec3i32};
use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::game_renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::render_objects::camera::RotatableCamera;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::physics_world::PhysicsWorld;
use crate::types::blocks::block::Block;
use crate::types::coordinates::core::{ChunkCoords, GlobalCoords};
use crate::types::dimension::Dimension;
use crate::types::item::ItemType;
use crate::types::player_object::PlayerObject;
use crate::utils::raycast::raycast;
use crate::utils::save_manager::SaveManager;
use crate::utils::world_generator::WorldGenerator;

pub enum PhysicsEvent {
    PlaceBlock(GlobalCoords, Block),
}

pub struct World<G: WorldGenerator> {
    pub player_object: PlayerObject,
    overworld: Dimension,
    overworld_generator: G,
    prev_player_chunk: Vector3<i32>,
    render_radius: i32,
    storage_radius: i32,
    overworld_save_manager: SaveManager,
    physics_world: PhysicsWorld,
    event_queue: Vec<PhysicsEvent>,
}

pub const BASE_RENDER_RADIUS: i32 = 1;

impl<G: WorldGenerator> World<G> {
    pub fn new(seed: u32) -> Self {
        let player_coords =
            PrecisePositionC32::raw_new(Vec3i32::new(0, 0, 0), Vec3f32::new(0.0, 128.0, 0.0))
                .normalized();
        Self {
            player_object: PlayerObject::new(
                player_coords,
                RotatableCamera::new(2000.0, 1200.0, 1.0),
            ),
            overworld: Dimension::new(),
            overworld_generator: G::new(seed),
            prev_player_chunk: Vector3::new(0, 0, 0),
            render_radius: BASE_RENDER_RADIUS,
            storage_radius: BASE_RENDER_RADIUS + 1,
            overworld_save_manager: SaveManager::new(
                ChunkCoords::from(player_coords.chunk).get_region(),
            ),
            physics_world: PhysicsWorld::default(),
            event_queue: Default::default(),
        }
    }

    pub fn init(&mut self) {
        let player_start_chunk = self.player_object.get_position().normalized().chunk;
        self.overworld_save_manager
            .shift_center(ChunkCoords::from(player_start_chunk).get_region());
        process_cube(
            |coords| {
                self.overworld.prepare_chunk(coords);
            },
            player_start_chunk,
            self.storage_radius,
        );

        process_cube(
            |coords| {
                self.overworld.show_chunk(coords);
            },
            player_start_chunk,
            self.render_radius,
        );
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.player_object
            .update(input_state, dt, &mut self.overworld, &mut self.event_queue);
        self.physics_world.update(dt, &self.overworld);

        self.handle_debug_input(input_state);

        self.update_player_chunk();
        self.player_raycast(input_state);
        self.physics_world.read_queue(&mut self.event_queue);
    }

    fn update_player_chunk(&mut self) {
        let new_player_chunk = self.player_object.get_position().chunk;
        if new_player_chunk != self.prev_player_chunk {
            self.update_chunks_state(new_player_chunk);
            let prev_player_region = ChunkCoords::from(self.prev_player_chunk).get_region();
            if ChunkCoords::from(new_player_chunk).get_region() != prev_player_region {
                self.overworld_save_manager.shift_center(prev_player_region);
            }
        }
        self.prev_player_chunk = new_player_chunk;
    }

    pub fn handle_debug_input(&mut self, input_state: &InputState) {
        let player_chunk = self.player_object.get_position().chunk;
        if input_state.is_just_pressed(KeyCode::F2) {
            println!("Чанк игрока: {}", player_chunk);
        }
        if input_state.is_just_pressed(KeyCode::F3) {
            println!(
                "Координаты: {}",
                self.player_object.physic_body.position.chunk.as_f32() * 32.0
                    + self.player_object.physic_body.position.in_chunk
            );
        }
        if input_state.is_just_pressed(KeyCode::F4) {
            let chunk_coords = ChunkCoords::from(player_chunk);
            self.unload_chunk(chunk_coords);
            self.overworld.save_chunks(&mut self.overworld_save_manager);
            self.prepare_chunk(chunk_coords);
            println!("Перезагрузка чанка");
        }
    }

    pub fn player_raycast(&mut self, input_state: &InputState) {
        let origin = self
            .player_object
            .physic_body
            .position
            .raw_add(PlayerObject::CAMERA_OFFSET);
        let direction = self.player_object.camera.lens.get_direction();

        let raycast_result = raycast(&self.overworld, origin, direction, 8.0);

        if let Some((phys_raycast, idx)) =
            self.physics_world.raycast_physics(origin, direction, 8.0)
        {
            if let Some(world_raycast) = raycast_result {
                if phys_raycast.distance < world_raycast.distance {
                    if input_state.is_mouse_just_pressed(MouseButton::Left) {
                        self.physics_world.set_block(
                            idx,
                            phys_raycast.target_block,
                            Block::default(),
                        );
                    }
                    if let ItemType::Block(block) = self.player_object.get_hand_item().item_type {
                        if input_state.is_mouse_just_pressed(MouseButton::Right) {
                            self.physics_world
                                .set_block(idx, phys_raycast.previous_block, block);
                        }
                    }
                } else {
                    if input_state.is_mouse_just_pressed(MouseButton::Left) {
                        self.overworld
                            .set_block_loaded(world_raycast.target_block, Block::default());
                    }
                    self.player_object.process_rmb(
                        input_state,
                        &mut self.overworld,
                        world_raycast,
                        &mut self.event_queue,
                    );
                }
            } else {
                if input_state.is_mouse_just_pressed(MouseButton::Left) {
                    self.physics_world
                        .set_block(idx, phys_raycast.target_block, Block::default());
                }
                if let ItemType::Block(block) = self.player_object.get_hand_item().item_type {
                    if input_state.is_mouse_just_pressed(MouseButton::Right) {
                        self.physics_world
                            .set_block(idx, phys_raycast.previous_block, block);
                    }
                }
            }
        } else {
            if let Some(raycast) = raycast_result {
                if input_state.is_mouse_just_pressed(MouseButton::Left) {
                    self.overworld
                        .set_block_loaded(raycast.target_block, Block::default());
                }
                self.player_object.process_rmb(
                    input_state,
                    &mut self.overworld,
                    raycast,
                    &mut self.event_queue,
                );
            }
        }
    }

    pub fn update_meshes(&mut self, renderer: &mut VkBackend) {
        self.overworld
            .prepare_chunks(&self.overworld_generator, &self.overworld_save_manager);
        self.overworld.save_chunks(&mut self.overworld_save_manager);

        self.overworld.update_chunk_meshes(renderer);
        self.player_object.update_inventory_meshes(renderer);
        self.physics_world.update_meshes(renderer);
    }

    pub fn show_chunk(&mut self, chunk: ChunkCoords) {
        self.overworld.show_chunk(chunk);
    }

    pub fn hide_chunk(&mut self, chunk: ChunkCoords) {
        self.overworld.hide_chunk(chunk);
    }

    pub fn prepare_chunk(&mut self, chunk: ChunkCoords) {
        self.overworld.prepare_chunk(chunk);
    }

    pub fn unload_chunk(&mut self, chunk: ChunkCoords) {
        self.overworld.unload_chunk(chunk);
    }

    pub fn update_chunks_state(&mut self, new_player_chunk: Vector3<i32>) {
        let visible_radius = self.render_radius;
        let load_radius = self.storage_radius;

        self.shift_cube_layers(
            self.prev_player_chunk,
            new_player_chunk,
            visible_radius,
            true,
        );

        self.shift_cube_layers(self.prev_player_chunk, new_player_chunk, load_radius, false);
    }

    fn shift_cube_layers(
        &mut self,
        old_center: Vector3<i32>,
        new_center: Vector3<i32>,
        radius: i32,
        is_graphics: bool,
    ) {
        let old_min = old_center - Vector3::new(radius, radius, radius);
        let old_max = old_center + Vector3::new(radius, radius, radius);
        let new_min = new_center - Vector3::new(radius, radius, radius);
        let new_max = new_center + Vector3::new(radius, radius, radius);

        let scan_min = old_min.min_cw(new_min);
        let scan_max = old_max.max_cw(new_max);

        for x in scan_min.x..=scan_max.x {
            for y in scan_min.y..=scan_max.y {
                for z in scan_min.z..=scan_max.z {
                    let coords = ChunkCoords::new(x, y, z);

                    let in_old = x >= old_min.x
                        && x <= old_max.x
                        && y >= old_min.y
                        && y <= old_max.y
                        && z >= old_min.z
                        && z <= old_max.z;

                    let in_new = x >= new_min.x
                        && x <= new_max.x
                        && y >= new_min.y
                        && y <= new_max.y
                        && z >= new_min.z
                        && z <= new_max.z;

                    if !in_old && in_new {
                        if is_graphics {
                            self.show_chunk(coords);
                        } else {
                            self.prepare_chunk(coords);
                        }
                    }

                    if in_old && !in_new {
                        if is_graphics {
                            self.hide_chunk(coords);
                        } else {
                            self.unload_chunk(coords);
                        }
                    }
                }
            }
        }
    }
}

pub fn process_cube<F: FnMut(ChunkCoords)>(mut function: F, center: Vector3<i32>, radius: i32) {
    for y in (center.y - radius)..=(center.y + radius) {
        for z in (center.z - radius)..=(center.z + radius) {
            for x in (center.x - radius)..=(center.x + radius) {
                function(ChunkCoords::new(x, y, z))
            }
        }
    }
}
