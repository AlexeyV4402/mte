use crate::dimension::Dimension;
use crate::player_object::PlayerObject;
use crate::raycast::raycast;
use crate::types::blocks::block::{Block, BlockType};

mod chunk;
mod config;
mod dimension;
mod network;
mod physics_body;
mod player_object;
mod raycast;
mod types;
mod utils;

use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use glam::Vec3;
use lib_io::user_io::InputState;
use lib_renderer::context::render_context::RenderContext;
use lib_renderer::renderer::block_grid_renderer::Renderer;
use types::coordinates;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalPosition;
use winit::event::{DeviceEvent, DeviceId, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::KeyCode::Escape;
use winit::window::WindowAttributes;

pub struct App {
    renderer: Option<Renderer>,
    input_state: InputState,
    last_time: Instant,
    paused: bool,
    dimension: Dimension,
    player_object: PlayerObject,
}

impl App {
    pub fn new() -> Self {
        Self {
            renderer: None,
            input_state: Default::default(),
            last_time: Instant::now(),
            paused: false,
            dimension: Dimension::new(),
            player_object: PlayerObject::new(Vec3::new(0.0, 35.0, 0.0)),
        }
    }
}

impl ApplicationHandler<Renderer> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = WindowAttributes::default()
            .with_inner_size(winit::dpi::LogicalSize::new(2000.0, 1200.0));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let gpu_context = pollster::block_on(RenderContext::new(window.clone())).unwrap();

        self.renderer = Some(pollster::block_on(Renderer::new(gpu_context)).unwrap());
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: Renderer) {
        self.renderer = Some(event);
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
        let dt = self.last_time.elapsed();
        self.last_time = Instant::now();
        self.input_state.handle_window_event(&event);

        let renderer = match &mut self.renderer {
            Some(canvas) => canvas,
            None => return,
        };

        let window = &renderer.render_context.window_contexts[0].window.clone();

        // Рабочий цикл

        let origin = renderer.camera_system.camera.position.into();
        let direction = renderer.camera_system.get_direction().into();

        // println!("{:?}", direction);

        // let start = Instant::now();
        let raycast_result = raycast(&self.dimension, origin, direction, 8.0);
        // println!("Поиск рейкаста: {} ms", start.elapsed().as_millis());

        if let Some(raycast) = raycast_result {
            if self.input_state.is_mouse_just_pressed(MouseButton::Left) {
                self.dimension
                    .set_block(raycast.target_block, Block::default());
            }
            if self.input_state.is_mouse_just_pressed(MouseButton::Right) {
                self.dimension
                    .set_block(raycast.previous_block, Block::from_type(BlockType::Dirt));
            }
        }

        self.dimension.update_chunk_meshes(renderer);

        // Конец рабочего цикла

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if !self.paused {
                    renderer.update(dt);
                    renderer.render().unwrap();
                }
            }
            _ => {}
        }

        let camera_system = &mut self.renderer.as_mut().unwrap().camera_system;
        camera_system.update(&self.input_state, dt);

        self.player_object.update(
            &self.input_state,
            dt,
            camera_system.get_direction(),
            &self.dimension,
        );

        camera_system.set_position(self.player_object.get_position() + Vec3::new(0.0, 0.7, 0.0));

        if self.input_state.is_just_pressed(Escape) {
            if !self.paused {
                if window.fullscreen().is_none() {
                    window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                }
                window
                    .set_cursor_grab(winit::window::CursorGrabMode::None)
                    .expect("Не удалось захватить курсор");
                // window.set_cursor_visible(false);
            } else {
                window
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                    .expect("Не удалось захватить курсор");
                window
                    .set_cursor_position(LogicalPosition::new(1280.0, 700.0))
                    .expect("msg");
                // window.set_cursor_visible(true);
                window.request_redraw();
            }
            self.paused = !self.paused;
        }

        self.input_state.update();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let renderer = match &mut self.renderer {
            Some(canvas) => canvas,
            None => return,
        };

        let window = &renderer.render_context.window_contexts[0].window.clone();
        // sleep(Duration::from_millis(16));
        window.request_redraw();
    }
}

pub fn main() -> anyhow::Result<()> {
    unsafe { std::env::set_var("WINIT_UNIX_BACKEND", "x11") };
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();

    event_loop.run_app(&mut app)?;
    Ok(())
}
