use std::ops::{Add, Mul};

pub mod operations;
pub mod operations_cw;
pub mod operations_hr;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3<T> {
    #[inline(always)]
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn euclidian_len_sq(self) -> T
    where
        T: Mul<Output = T> + Add<Output = T> + Copy,
    {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Vector3<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}
