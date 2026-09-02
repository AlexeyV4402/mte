use std::ops::{Add, Mul};

use crate::math::vectors::Vector;
use crate::math::vectors::vec4::core::Vector4;

pub mod operations;
pub mod operations_all;
pub mod operations_cw;
pub mod operations_hr;

#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3<T> {
    #[inline(always)]
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn euclidian_len_sq(self) -> T
    where
        T: Mul<Output = T> + Add<Output = T> + Copy,
    {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline(always)]
    pub fn to_array(self) -> [T; 3] {
        [self.x, self.y, self.z]
    }
}

impl<T: Default> Vector3<T> {
    #[inline(always)]
    pub fn to_vec4_left(self) -> Vector4<T> {
        Vector4::new(self.x, self.y, self.z, T::default())
    }

    #[inline(always)]
    pub fn to_vec4_right(self) -> Vector4<T> {
        Vector4::new(T::default(), self.x, self.y, self.z)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Vector3<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl<T> Vector for Vector3<T> {}
