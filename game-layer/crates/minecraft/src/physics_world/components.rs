use glam::{Mat3, Mat4, Quat, Vec3};
use lib_core::math::vectors::custom::PrecisePositionC32;
use lib_core::math::vectors::vec3::core::Vector3;

use crate::physics_world::physics_construction::PhysicsConstruction;
use crate::types::blocks::block::Block;
use crate::types::coordinates::core::GlobalCoords;

#[derive(Default)]
pub struct Components {
    // --- Линейное движение (Linear Dynamics) ---
    pub positions: Vec<PrecisePositionC32>,
    pub velocities: Vec<Vec3>,    // Линейная скорость (м/с)
    pub accelerations: Vec<Vec3>, // Линейное ускорение (м/с²)
    pub forces: Vec<Vec3>,        // Сумма всех линейных сил (Ньютоны), сбрасывается каждый кадр

    // --- Вращательное движение (Angular Dynamics) ---
    pub orientations: Vec<Quat>, // Ориентация тела в пространстве (Кватернион)
    pub angular_velocities: Vec<Vec3>, // Угловая скорость (радианы/с, вектор задает ось и скорость)
    pub torques: Vec<Vec3>, // Сумма всех крутящих моментов (Ньютон-метры), сбрасывается каждый кадр

    // --- Масс-инерционные характеристики (Mass & Inertia) ---
    pub masses: Vec<f32>, // Масса тела (кг). Если тело статично, можно использовать f32::INFINITY
    pub inverse_masses: Vec<f32>, // 1.0 / mass. В физике чаще используют инверсию, так как на нее умножают, а не делят (ускоряет расчеты и убирает деление на 0)
    pub inverse_inertia_tensors: Vec<Mat3>, // Инвертированный тензор инерции (3х3 матрица). Описывает, как тяжело вращать тело по разным осям.

    pub construct_data: Vec<PhysicsConstruction>,
}

impl Components {
    pub fn push_default(&mut self, coords: GlobalCoords, block: Block) -> usize {
        self.positions.push(PrecisePositionC32::raw_new(
            coords.get_chunk().0,
            coords.get_local().0.as_f32() + Vector3::new(0.5, 0.5, 0.5),
        ));

        self.velocities.push(Vec3::ZERO);
        self.accelerations.push(Vec3::ZERO);
        self.forces.push(Vec3::ZERO);
        self.orientations.push(Quat::default());
        self.angular_velocities.push(Vec3::new(0.0, 0.0, 0.0));
        self.torques.push(Vec3::ZERO);
        self.masses.push(1.0);
        self.inverse_masses.push(1.0);
        self.inverse_inertia_tensors.push(Mat3::IDENTITY);
        self.construct_data
            .push(PhysicsConstruction::one_block(block));

        self.velocities.len() - 1
    }

    pub fn get_matrix(&self, idx: usize) -> [[f32; 4]; 4] {
        let translation: Vec3 = self.positions[idx].in_chunk.into();

        let rotation: Quat = self.orientations[idx];

        let local_mass_center: Vec3 = self.construct_data[idx].local_mass_center;

        let model_matrix = Mat4::from_rotation_translation(rotation, translation)
            * Mat4::from_translation(-local_mass_center);

        model_matrix.to_cols_array_2d()
    }

    pub fn get_vector(&self, idx: usize) -> [i32; 4] {
        self.positions[idx].chunk.to_vec4_left().to_array()
    }
}
