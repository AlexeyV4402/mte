use std::time::Duration;

use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;
use lib_core::math::vectors::vec3::types::{Vec3f32, Vec3i32};
use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::render_objects::camera::RotatableCamera;
use winit::keyboard::KeyCode;

use crate::types::coordinates::core::ChunkCoords;
use crate::types::dimension::Dimension;
use crate::types::player_object::PlayerObject;
use crate::utils::save_manager::SaveManager;
use crate::world_generator::WorldGenerator;

pub struct World<G: WorldGenerator> {
    pub player_object: PlayerObject,
    overworld: Dimension,
    overworld_generator: G,
    prev_player_chunk: Vector3<i32>,
    render_radius: i32,
    storage_radius: i32,
    overworld_save_manager: SaveManager,
}

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
            render_radius: 2,
            storage_radius: 3,
            overworld_save_manager: SaveManager::new(
                ChunkCoords::from(player_coords.chunk).get_region(),
            ),
        }
    }

    pub fn init(&mut self) {
        let player_start_chunk = self.player_object.get_position().normalized().chunk;
        self.overworld_save_manager
            .shift_center(ChunkCoords::from(player_start_chunk).get_region());
        for y in (player_start_chunk.y - self.storage_radius)
            ..=(player_start_chunk.y + self.storage_radius)
        {
            for z in (player_start_chunk.z - self.storage_radius)
                ..=(player_start_chunk.z + self.storage_radius)
            {
                for x in (player_start_chunk.x - self.storage_radius)
                    ..=(player_start_chunk.x + self.storage_radius)
                {
                    self.overworld.prepare_chunk(ChunkCoords::new(x, y, z));
                }
            }
        }
        for y in -(self.render_radius - player_start_chunk.y)
            ..=(self.render_radius + player_start_chunk.y)
        {
            for z in -(self.render_radius - player_start_chunk.z)
                ..=(self.render_radius + player_start_chunk.z)
            {
                for x in -(self.render_radius - player_start_chunk.x)
                    ..=(self.render_radius + player_start_chunk.x)
                {
                    self.overworld.show_chunk(ChunkCoords::new(x, y, z));
                }
            }
        }
    }

    pub fn update(&mut self, dt: Duration, input_state: &InputState) {
        self.player_object
            .update(input_state, dt, &mut self.overworld);
        let player_chunk = self.player_object.get_position().chunk;
        if input_state.is_just_pressed(KeyCode::F2) {
            println!("Чанк игрока: {}", player_chunk);
        }
        if input_state.is_just_pressed(KeyCode::F3) {
            let chunk_coords = ChunkCoords::from(player_chunk);
            self.unload_chunk(chunk_coords);
            self.overworld.save_chunks(&mut self.overworld_save_manager);
            self.prepare_chunk(chunk_coords);
        }
        if player_chunk != self.prev_player_chunk {
            self.update_chunks_state(player_chunk);
            let player_region = ChunkCoords::from(self.prev_player_chunk).get_region();
            if ChunkCoords::from(player_chunk).get_region() != player_region {
                self.overworld_save_manager.shift_center(player_region);
            }
        }
        self.prev_player_chunk = player_chunk;
    }

    pub fn update_meshes(&mut self, renderer: &mut Renderer) {
        self.overworld
            .prepare_chunks(&self.overworld_generator, &self.overworld_save_manager);
        self.overworld.save_chunks(&mut self.overworld_save_manager);
        self.overworld.update_chunk_meshes(renderer);
        self.player_object.update_inventory_meshes(renderer);
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
