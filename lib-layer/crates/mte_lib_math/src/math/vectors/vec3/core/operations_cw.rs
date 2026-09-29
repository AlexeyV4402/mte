use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr};

use super::Vector3;

/// Component-wise methods
impl<T> Vector3<T> {
    #[inline(always)]
    pub fn not_cw(self) -> Vector3<T::Output>
    where
        T: Not,
    {
        Vector3 {
            x: !self.x,
            y: !self.y,
            z: !self.z,
        }
    }

    #[inline(always)]
    pub fn div_cw(self, other: Self) -> Vector3<T::Output>
    where
        T: Div<T>,
    {
        Vector3 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
        }
    }

    #[inline(always)]
    pub fn mul_cw(self, other: Self) -> Vector3<T::Output>
    where
        T: Mul<T>,
    {
        Vector3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }

    #[inline(always)]
    pub fn min_cw(self, other: Self) -> Self
    where
        T: Ord,
    {
        Vector3 {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    #[inline(always)]
    pub fn max_cw(self, other: Self) -> Self
    where
        T: Ord,
    {
        Vector3 {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }
}
