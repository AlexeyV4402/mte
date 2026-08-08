use std::f32::consts::PI;
use std::time::Duration;

use glam::{Mat4, Vec3};
use lib_io::user_io::InputState;
use wgpu::util::DeviceExt;
use wgpu::BindGroupLayout;
use winit::keyboard::KeyCode;

use crate::context::render_context::RenderContext;

const SPEED: f32 = 5.0;

pub struct CameraSystem {
    pub camera: CameraPos,
    pub controller: CameraController,
    pub projection: CameraProj,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl CameraSystem {
    pub fn new(
        render_context: &RenderContext,
        camera_bind_group_layout: &BindGroupLayout
    ) -> Self {
        let gpu_context = &render_context.gpu_contexts[0];
        let config = &render_context.window_contexts[0].config;

        let camera = CameraPos {
            position: Vec3 {
                x: 3.0,
                y: 0.0,
                z: -30.0,
            },
            yaw_rad: 0.0,
            pitch_rad: 0.0,
        };
        let projection = CameraProj::new(config.width, config.height, PI / 2.0, 0.1, 500.0);

        let camera_buffer =
            gpu_context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera Buffer"),
                    contents: bytemuck::cast_slice(&[CameraUniform::IDENT]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let camera_bind_group = gpu_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    },
                ],
                label: Some("camera_bind_group"),
            });

        Self {
            camera,
            controller: CameraController::new(SPEED, 1.0),
            projection,
            buffer: camera_buffer,
            bind_group: camera_bind_group,
        }
    }

    pub fn update(&mut self, input_state: &InputState, dt: Duration) {
        self.controller
            .update_camera(&mut self.camera, input_state, dt);
    }

    pub fn get_direction(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.camera.yaw_rad.sin_cos();
        let (pitch_sin, pitch_cos) = self.camera.pitch_rad.sin_cos();
        Vec3::new(pitch_cos * yaw_sin, pitch_sin, pitch_cos * yaw_cos).normalize_or_zero()
    }

    pub fn get_uniform(&self) -> CameraUniform {
        let view = self.camera.calc_matrix();
        // let view = Mat4::IDENTITY;
        let proj = self.projection.calc_matrix();
        // let proj = Mat4::IDENTITY;
        let view_proj = proj * view;

        CameraUniform {
            // view_position: self.camera.position.extend(1.0).into(),
            // view: view.to_cols_array_2d(),
            view_proj: view_proj.to_cols_array_2d(),
            // view_proj: Mat4::IDENTITY.transpose().to_cols_array_2d(),

            // inv_proj: proj.safe_inverse().to_cols_array_2d(),
            // inv_view: view.safe_inverse().to_cols_array_2d(),
        }
    }
}

#[derive(Debug)]
pub struct CameraPos {
    pub position: Vec3,
    pub yaw_rad: f32,
    pub pitch_rad: f32,
}

impl CameraPos {
    /// Calculate view matrix
    pub fn calc_matrix(&self) -> Mat4 {
        let (pitch_sin, pitch_cos) = self.pitch_rad.sin_cos();
        let (yaw_sin, yaw_cos) = self.yaw_rad.sin_cos();

        // ИСПРАВЛЕНО: Привели базис осей X и Z в соответствие с контроллером
        // Для yaw = 0 и pitch = 0 вектор станет (0.0, 0.0, 1.0) - взгляд строго вперед по Z
        let forward =
            Vec3::new(pitch_cos * yaw_sin, pitch_sin, pitch_cos * yaw_cos).normalize_or_zero();

        Mat4::look_to_lh(self.position, forward, Vec3::Y)
    }
}

pub struct CameraProj {
    aspect: f32,
    fovy_rad: f32,
    znear: f32,
    zfar: f32,
}

impl CameraProj {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy_rad: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_lh(self.fovy_rad, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self { speed, sensitivity }
    }

    pub fn update_camera(
        &mut self,
        camera: &mut CameraPos,
        input_state: &InputState,
        dt: Duration,
    ) {
        let dt = dt.as_secs_f32();
        if dt <= 0.0 {
            return;
        }

        // 1. Поворот мыши (Чистый плюс: движение мыши вправо увеличивает Yaw)
        camera.yaw_rad += input_state.raw_mouse_delta.x * self.sensitivity * dt;

        camera.pitch_rad -= input_state.raw_mouse_delta.y * self.sensitivity * dt;
        camera.pitch_rad = camera.pitch_rad.clamp(-1.55, 1.55);

        // 2. Правильная левосторонняя тригонометрия
        let (yaw_sin, yaw_cos) = camera.yaw_rad.sin_cos();
        let (pitch_sin, pitch_cos) = camera.pitch_rad.sin_cos();

        // Направление Вперед (Z уходит вглубь экрана при yaw = 0)
        let forward =
            Vec3::new(pitch_cos * yaw_sin, pitch_sin, pitch_cos * yaw_cos).normalize_or_zero();

        // Направление Вправо (В ЛЕВОЙ системе: Y.cross(Forward) дает Вправо)
        let right = Vec3::Y.cross(forward).normalize_or_zero();

        // 3. Branchless получение осей
        let get_axis = |key: KeyCode| input_state.is_down(key) as u32 as f32;

        let move_forward = get_axis(KeyCode::KeyW) - get_axis(KeyCode::KeyS);
        let move_right = get_axis(KeyCode::KeyD) - get_axis(KeyCode::KeyA);
        let move_up = get_axis(KeyCode::Space) - get_axis(KeyCode::ControlLeft);

        self.speed = get_axis(KeyCode::ShiftLeft) * SPEED + SPEED;

        // 4. Применение движения (Везде строгие ПЛЮСЫ)
        camera.position += forward * move_forward * self.speed * dt;
        camera.position += right * move_right * self.speed * dt;
        camera.position.y += move_up * self.speed * dt;

        // Скролл (Движение вперед при положительном скролле)
        camera.position +=
            forward * input_state.mouse_scroll_delta * self.speed * self.sensitivity * dt;
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // pub view_position: [f32; 4],
    // pub view: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    // pub inv_proj: [[f32; 4]; 4],
    // pub inv_view: [[f32; 4]; 4],
}

impl CameraUniform {
    pub const IDENT: Self = Self {
        // view_position: [0.0; 4],
        // view: Mat4::IDENTITY.to_cols_array_2d(),
        view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        // inv_proj: Mat4::IDENTITY.to_cols_array_2d(),
        // inv_view: Mat4::IDENTITY.to_cols_array_2d(),
    };
}
