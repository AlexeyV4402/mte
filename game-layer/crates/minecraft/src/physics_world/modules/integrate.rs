use glam::{Quat, Vec3};

use crate::physics_world::components::Components;
use crate::physics_world::modules::render::RenderModule;

#[derive(Default)]
pub struct IntegrateModule;

impl IntegrateModule {
    #[inline(always)]
    pub fn system_apply_gravity(&self, components: &mut Components) {
        let gravity = Vec3::new(0.0, -9.81, 0.0);

        for (force, &inv_mass) in components
            .forces
            .iter_mut()
            .zip(components.inverse_masses.iter())
        {
            if inv_mass > 0.0 {
                let mass = 1.0 / inv_mass;
                *force += gravity * mass;
            }
        }
    }

    #[inline(always)]
    pub fn system_integrate_velocities(
        &self,
        components: &mut Components,
        render_module: &mut RenderModule,
        delta_time: f32,
    ) {
        let loops = components.velocities.len();
        for i in 0..loops {
            let inv_mass = components.inverse_masses[i];
            if inv_mass > 0.0 {
                components.accelerations[i] = components.forces[i] * inv_mass;
                components.velocities[i] += components.accelerations[i] * delta_time;
                render_module.update_queue.push(i);
            }
        }
    }

    #[inline(always)]
    pub fn system_integrate_positions(&mut self, components: &mut Components, delta_time: f32) {
        for (pos, velocity) in components
            .positions
            .iter_mut()
            .zip(components.velocities.iter())
        {
            let displacement = *velocity * delta_time;
            // if (pos.chunk.as_f32() * 32.0 + pos.in_chunk + displacement.into()).y <= 0.5 {
            //     continue;
            // }

            pos.in_chunk += displacement.into();

            pos.normalize();
        }
    }

    #[inline(always)]
    pub fn system_integrate_orientations(&self, components: &mut Components, delta_time: f32) {
        for (quat, velocity) in components
            .orientations
            .iter_mut()
            .zip(components.angular_velocities.iter())
        {
            let angular_velocity = velocity;
            if angular_velocity != &Vec3::ZERO {
                let rotation_delta = Quat::from_axis_angle(
                    angular_velocity.normalize(),
                    angular_velocity.length() * delta_time,
                );
                *quat = (rotation_delta * (*quat)).normalize();
            }
        }
    }

    #[inline(always)]
    pub fn system_clear_forces(&self, components: &mut Components) {
        for force in components.forces.iter_mut() {
            *force = Vec3::ZERO;
        }
    }
}
