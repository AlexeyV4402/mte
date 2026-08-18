use std::ops::{Add, Mul, Sub};

use super::Vector3;

impl<T> From<(T, T, T)> for Vector3<T> {
    #[inline(always)]
    fn from(v: (T, T, T)) -> Self {
        Vector3::new(v.0, v.1, v.2)
    }
}

impl<T> Into<(T, T, T)> for Vector3<T> {
    #[inline(always)]
    fn into(self) -> (T, T, T) {
        (self.x, self.y, self.z)
    }
}

impl<T: Add<Output = T>> Add for Vector3<T> {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Vector3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl<T: Sub<Output = T>> Sub for Vector3<T> {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Vector3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vector3<T> {
    type Output = Self;

    #[inline(always)]
    fn mul(self, scalar: T) -> Self::Output {
        Vector3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}
