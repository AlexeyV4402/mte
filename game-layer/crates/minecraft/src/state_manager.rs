use std::time::{Duration, Instant};

use chrono::Timelike;
use glam::Vec2;
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::types::{Vec3f32, Vec3i32};
use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::game_data::VkInGameData;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::render_objects::camera::RotatableLens;
use lib_renderer::renderer::block_grid_renderer::types::RendererCreateArgs;
use winit::dpi::PhysicalSize;

use crate::states::in_game::InGameState;
use crate::states::main_menu::MainMenuState;
use crate::types::blocks::block::{
    BLOCK_PROPERTIES_REGISTRY, CUBE_LINES, REGISTERED_TEXTURES_COUNT
};
use crate::types::world::World;
use crate::utils::save_manager::save_manager::{SaveManager, WorldHeader};
use crate::utils::world_generator::SuperSimplexGenerator;

pub enum State {
    MainMenu(MainMenuState),
    InGame(InGameState),
    None,
}

pub enum GlobalEvent {
    StartGame,
}

pub struct StateManager {
    pub state: State,
    global_event_queue: Vec<GlobalEvent>,
    save_manager: SaveManager,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: State::None,
            global_event_queue: Vec::new(),
            save_manager: SaveManager {},
        }
    }

    pub fn setup_main_menu(&mut self, renderer: &VkBackend) {
        let device = &renderer.device;
        let mem_properties = &renderer.memory_prop;
        let render_pass = renderer.render_pass;
        let state = MainMenuState::new(device, mem_properties, render_pass);
        self.state = State::MainMenu(state);
    }

    pub fn setup_game(&mut self, renderer: &VkBackend) {
        let data_create_args = RendererCreateArgs {
            block_properties: bytemuck::cast_slice(&BLOCK_PROPERTIES_REGISTRY),
            layer_count: REGISTERED_TEXTURES_COUNT as u32,
            outline_vertices: bytemuck::cast_slice(&CUBE_LINES),
        };

        let device = &renderer.device;
        let mem_properties = &renderer.memory_prop;
        let render_pass = renderer.render_pass;
        let graphics_queue = renderer.graphics_queue;
        let cmd_pool = renderer.cmd_pool;

        let vk_data = VkInGameData::new(
            device,
            mem_properties,
            render_pass,
            graphics_queue,
            cmd_pool,
            data_create_args,
        );

        let datetime = chrono::NaiveDateTime::MIN;

        // let seed = datetime.nanosecond() % 100;
        let seed = 32;

        let world_header = WorldHeader {
            name: format!("{}", seed),
            seed,
            color: SaveManager::seed_to_rgb(seed),
            player_dim: 0,
            player_pos: PrecisePositionC32::raw_new(
                Vec3i32::new(0, 0, 0),
                Vec3f32::new(0.0, 128.0, 0.0),
            )
            .normalized(),
        };
        let mut world: World<SuperSimplexGenerator> = World::new(seed);

        world.init();

        let state = std::mem::replace(&mut self.state, State::None);
        if let State::MainMenu(main_menu) = state {
            main_menu.destroy(device);
        }

        self.state = State::InGame(InGameState { vk_data, world })
    }

    pub fn redraw_requested(&mut self, renderer: &mut VkBackend) {
        let events: Vec<GlobalEvent> = self.global_event_queue.drain(..).collect();

        for global_event in events {
            match global_event {
                GlobalEvent::StartGame => {
                    self.setup_game(renderer);
                    break;
                }
            }
        }
        match &mut self.state {
            State::MainMenu(main_menu_state) => {
                let vk_data = &mut main_menu_state.vk_data;

                renderer.begin_frame();
                main_menu_state.main_menu.update_meshes(vk_data, renderer);
                renderer.end_frame(vk_data).unwrap();
            }
            State::InGame(in_game_state) => {
                let vk_data = &mut in_game_state.vk_data;

                renderer.begin_frame();
                in_game_state.world.update_meshes(vk_data, renderer);

                vk_data.update_camera(in_game_state.world.player_object.get_camera_world_uniform());

                renderer.end_frame(vk_data).unwrap();
            }
            State::None => todo!(),
        }
    }

    pub fn about_to_wait(&mut self, dt: Duration, input_state: &InputState, screen_size: Vec2) {
        match &mut self.state {
            State::MainMenu(main_menu_state) => {
                main_menu_state.main_menu.update(
                    input_state,
                    &mut self.global_event_queue,
                    screen_size,
                );
            }
            State::InGame(in_game_state) => {
                in_game_state.world.update(dt, input_state);
            }
            State::None => todo!(),
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>, renderer: &mut VkBackend) {
        match &mut self.state {
            State::MainMenu(_) => {
                renderer.resize(size.width, size.height);
            }
            State::InGame(in_game_state) => {
                in_game_state.world.player_object.camera.lens =
                    RotatableLens::new(size.width as f32, size.height as f32);
                renderer.resize(size.width, size.height);
            }
            State::None => todo!(),
        }
    }
}
