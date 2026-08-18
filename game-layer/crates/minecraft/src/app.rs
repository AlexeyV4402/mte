use crate::raycast::raycast;
use crate::types;
use crate::types::blocks::block::{
    BLOCK_PROPERTIES_REGISTRY, Block, REGISTERED_TEXTURES_COUNT
};
use crate::types::coordinates::core::{ChunkCoords, GlobalCoords};
use crate::types::dimension::Dimension;
use crate::types::player_object::PlayerObject;

use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use glam::{I64Vec3, IVec3, Vec3};
use lib_io::user_io::InputState;
use lib_renderer::context::render_context::RenderContext;
use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::renderer::RendererCreateArgs;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalPosition;
use winit::event::{DeviceEvent, DeviceId, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
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

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        };
        #[allow(unused_mut)]
        let mut window_attributes = WindowAttributes::default()
            .with_inner_size(winit::dpi::LogicalSize::new(2000.0, 1200.0));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let render_context = pollster::block_on(RenderContext::new(window.clone())).unwrap();

        let renderer_create_args = RendererCreateArgs {
            block_properties: bytemuck::cast_slice(&BLOCK_PROPERTIES_REGISTRY),
            layer_count: REGISTERED_TEXTURES_COUNT as u32,
        };

        self.renderer =
            Some(pollster::block_on(Renderer::new(render_context, renderer_create_args)).unwrap());
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

        let renderer = match &mut self.renderer {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if !self.paused {
                    renderer.update();
                    renderer.render().unwrap();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let dt = self.last_time.elapsed();
        self.last_time = Instant::now();

        let renderer = match &mut self.renderer {
            Some(canvas) => canvas,
            None => return,
        };

        let window = &renderer.render_context.window_contexts[0].window.clone();

        renderer.camera_system.update(&self.input_state, dt);

        self.player_object.update(
            &self.input_state,
            dt,
            renderer.camera_system.get_direction(),
            &self.dimension,
        );

        let origin = renderer.camera_system.camera.position.into();
        let direction = renderer.camera_system.get_direction().into();

        // let start = Instant::now();
        let raycast_result = raycast(&self.dimension, origin, direction, 8.0);
        // println!("Поиск рейкаста: {} ms", start.elapsed().as_millis());

        if let Some(raycast) = raycast_result {
            if self.input_state.is_mouse_just_pressed(MouseButton::Left) {
                self.dimension
                    .set_block(raycast.target_block, Block::default());
            }
            if let types::item::ItemType::Block(block) =
                self.player_object.get_hand_item().item_type
            {
                if self.input_state.is_mouse_just_pressed(MouseButton::Right) {
                    self.dimension.set_block(raycast.previous_block, block);
                }
            }
        }
        
        let camera_position_f32 = self.player_object.get_position() + Vec3::new(0.0, 0.7, 0.0);
        // println!("camera_position_f32_1: {:?}", camera_position_f32);

        // // 2. ИСПРАВЛЕНИЕ БАГА: Считаем центр (координату чанка) строго через .floor()
        // // .floor() превратит -0.1 в -1.0, а 35.7 в 35.0.
        // let chunk_x = (camera_position_f32.x / 32.0).floor() as i32;
        // let chunk_y = (camera_position_f32.y / 32.0).floor() as i32;
        // let chunk_z = (camera_position_f32.z / 32.0).floor() as i32;

        // let relate_center = ChunkCoords::from((chunk_x, chunk_y, chunk_z)); // Твой тип координат чанка

        // // 3. Вычитаем из позиции f32 абсолютное пиксельное смещение этого чанка
        // // Теперь для отрицательных координат локальная позиция всегда будет строго от 0.0 до 32.0
        // camera_position_f32.x -= (chunk_x * 32) as f32;
        // camera_position_f32.y -= (chunk_y * 32) as f32;
        // camera_position_f32.z -= (chunk_z * 32) as f32;
        // println!("camera_position_f32_2: {:?}", camera_position_f32);
        // println!("relate_center: {:?}", relate_center);

        // 4. Пробрасываем в рендерер и инвентарь
        self.dimension.update_chunk_meshes(renderer);
        self.player_object.update_inventory_meshes(renderer);


        renderer.camera_system.set_position(camera_position_f32);

        if self.input_state.is_just_pressed(Escape) {
            if !self.paused {
                if window.fullscreen().is_none() {
                    window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                }
                match window.set_cursor_grab(winit::window::CursorGrabMode::None) {
                    Ok(_) => {}
                    Err(err) => println!("{}", err),
                };
                // window.set_cursor_visible(false);
            } else {
                match window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
                    Ok(_) => {}
                    Err(err) => println!("{}", err),
                };
                window
                    .set_cursor_position(LogicalPosition::new(1280.0, 700.0))
                    .expect("msg");
                // window.set_cursor_visible(true);
                window.request_redraw();
            }
            self.paused = !self.paused;
        }

        self.input_state.update();

        window.request_redraw();
    }
}