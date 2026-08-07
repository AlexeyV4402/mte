use std::ops::{Deref, DerefMut};

use bitcode::{Decode, Encode};
use glam::{Mat4, Vec3};

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct Position(pub Vec3);

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct Velocity(pub Vec3);

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct Acceleration(pub Vec3);

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct EntityId(pub u32, pub u32);

impl Position {
    fn get_mat(&self) -> Mat4 {
        Mat4::IDENTITY
    }
}

impl Deref for Position {
    type Target = glam::Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Position {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Velocity {
    type Target = glam::Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Velocity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Acceleration {
    type Target = glam::Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Acceleration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Encode, Decode, PartialEq, Debug, Eq, Ord, PartialOrd)]
pub struct MeshHandle(pub u32);
