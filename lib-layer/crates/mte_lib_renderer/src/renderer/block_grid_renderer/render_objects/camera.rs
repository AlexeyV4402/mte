use std::f32::consts::PI;
use std::time::Duration;

use glam::{Mat4, Quat, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_io::user_io::InputState;

pub struct RotatableLens {
    yaw_rad: f32,
    pitch_rad: f32,

    pub aspect: f32,
    fovy_rad: f32,
    znear: f32,
    zfar: f32,
}

pub struct RotatableCamera {
    pub lens: RotatableLens,
    pub rotator: CameraRotator,
}

impl RotatableLens {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            aspect: width / height,
            fovy_rad: PI / 2.0,
            znear: 0.1,
            zfar: 500.0,
        }
    }

    pub fn get_direction(&self) -> Vec3 {
        let (yaw_sin, yaw_cos) = self.yaw_rad.sin_cos();
        let (pitch_sin, pitch_cos) = self.pitch_rad.sin_cos();
        Vec3::new(pitch_cos * yaw_sin, pitch_sin, pitch_cos * yaw_cos).normalize_or_zero()
    }

    pub fn get_proj_mat(&self) -> Mat4 {
        glam::camera::lh::proj::vulkan::perspective(
            self.fovy_rad,
            self.aspect,
            self.znear,
            self.zfar,
        )
    }

    pub fn get_view_rotation_mat(&self) -> Mat4 {
        let yaw_quat = Quat::from_axis_angle(Vec3::Y, self.yaw_rad);
        let pitch_quat = Quat::from_axis_angle(Vec3::X, -self.pitch_rad);

        let camera_rotation = yaw_quat * pitch_quat;

        let view_rotation_quat = camera_rotation.conjugate();

        Mat4::from_quat(view_rotation_quat)
    }

    pub fn get_world_uniform(&self, position: PrecisePositionC32) -> WorldCameraUniform {
        let view_rotation = self.get_view_rotation_mat().to_cols_array_2d();
        let proj_matrix = self.get_proj_mat().to_cols_array_2d();

        WorldCameraUniform {
            proj_matrix,
            view_rotation,
            camera_chunk: position.chunk.to_vec4_left().to_array(),
            camera_in_chunk_position: position.in_chunk.to_vec4_left().to_array(),
        }
    }
}

impl RotatableCamera {
    pub fn new(width: f32, height: f32, sensitivity: f32) -> Self {
        Self {
            lens: RotatableLens::new(width, height),
            rotator: CameraRotator::new(sensitivity),
        }
    }

    pub fn update(&mut self, input_state: &InputState, dt: Duration) {
        self.rotator.update_camera(
            &mut self.lens.yaw_rad,
            &mut self.lens.pitch_rad,
            input_state,
            dt,
        );
    }
}

pub struct CameraRotator {
    sensitivity: f32,
}

impl CameraRotator {
    pub fn new(sensitivity: f32) -> Self {
        Self { sensitivity }
    }

    #[inline]
    pub fn update_camera(
        &mut self,
        yaw_rad: &mut f32,
        pitch_rad: &mut f32,
        input_state: &InputState,
        dt: Duration,
    ) {
        let dt = dt.as_secs_f32();
        if dt <= 0.0 {
            return;
        }

        *yaw_rad += input_state.raw_mouse_delta.x * self.sensitivity * dt;

        *pitch_rad -= input_state.raw_mouse_delta.y * self.sensitivity * dt;
        *pitch_rad = pitch_rad.clamp(-1.55, 1.55);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldCameraUniform {
    pub proj_matrix: [[f32; 4]; 4],
    pub view_rotation: [[f32; 4]; 4],
    pub camera_chunk: [i32; 4],
    pub camera_in_chunk_position: [f32; 4],
}
