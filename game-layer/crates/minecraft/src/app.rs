use std::sync::Arc;
use std::time::Instant;

use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::render_objects::camera::RotatableLens;
use lib_renderer::renderer::block_grid_renderer::types::RendererCreateArgs;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode::{Escape, F1, F11};
use winit::window::{CursorGrabMode, Window, WindowAttributes};

use crate::types::blocks::block::{
    BLOCK_PROPERTIES_REGISTRY, CUBE_LINES, REGISTERED_TEXTURES_COUNT
};
use crate::types::world::World;
use crate::world_generator::SuperSimplexGenerator;

pub struct App {
    vk_backend: Option<VkBackend>,
    input_state: InputState,
    last_time: Instant,
    paused: bool,
    world: World<SuperSimplexGenerator>,
    window_state: WindowState,
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            vk_backend: None,
            input_state: Default::default(),
            last_time: Instant::now(),
            paused: false,
            world: World::new(32),
            window_state: WindowState::default(),
            window: None,
        }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // if self.renderer.is_some() {
        //     return;
        // };
        let window_attributes = WindowAttributes::default()
            .with_inner_size(winit::dpi::LogicalSize::new(2000.0, 1200.0));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // let render_context = pollster::block_on(RenderContext::new(window.clone())).unwrap();

        let renderer_create_args = RendererCreateArgs {
            block_properties: bytemuck::cast_slice(&BLOCK_PROPERTIES_REGISTRY),
            layer_count: REGISTERED_TEXTURES_COUNT as u32,
            outline_vertices: bytemuck::cast_slice(&CUBE_LINES),
        };

        // self.renderer =
        //     Some(pollster::block_on(Renderer::new(render_context, renderer_create_args)).unwrap());

        self.world.init();
        self.vk_backend = Some(VkBackend::new(window.clone(), renderer_create_args));

        self.window = Some(window);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.input_state.handle_device_event(&event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.last_time = Instant::now();
        self.input_state.handle_window_event(&event);

        let renderer = match &mut self.vk_backend {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.world.player_object.camera.lens = RotatableLens::new(size.width as f32, size.height as f32);
                    renderer.resize(
                    size.width,
                    size.height,
                )
            }

            WindowEvent::RedrawRequested => {
                if !self.paused {
                    renderer.begin_frame();
                    self.world.update_meshes(renderer);
                    renderer.update_camera(self.world.player_object.get_camera_world_uniform());
                    renderer.end_frame(self.world.player_object.get_camera_hand_uniform()).unwrap();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let dt = self.last_time.elapsed();
        self.last_time = Instant::now();

        let window = match &self.window {
            Some(window) => window,
            None => return,
        };

        self.world.update(dt, &self.input_state);

        if self.input_state.is_just_pressed(F11) {
            if window.fullscreen().is_none() {
                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            } else {
                window.set_fullscreen(None);
            }
        }

        if self.input_state.is_just_pressed(F1) {
            match self.window_state.grab_mode {
                CursorGrabMode::None => {
                    match window.set_cursor_grab(CursorGrabMode::Locked) {
                        Ok(_) => {}
                        Err(err) => println!("{}", err),
                    };
                    self.window_state.grab_mode = CursorGrabMode::Locked;
                    window.set_cursor_visible(false);
                }
                CursorGrabMode::Confined => {
                    println!("Я хз, чё делать.")
                }
                CursorGrabMode::Locked => {
                    match window.set_cursor_grab(CursorGrabMode::None) {
                        Ok(_) => {}
                        Err(err) => println!("{}", err),
                    };
                    self.window_state.grab_mode = CursorGrabMode::None;
                    window.set_cursor_visible(true);
                }
            }
        }

        if self.input_state.is_just_pressed(Escape) {
            self.paused = !self.paused;
        }

        self.input_state.update();

        window.request_redraw();
    }
}

pub struct WindowState {
    grab_mode: CursorGrabMode,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            grab_mode: CursorGrabMode::None,
        }
    }
}
