use std::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};

use super::Vector3;

/// Component-wise methods
impl<T> Vector3<T> {
    #[inline(always)]
    pub fn shr<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        T: Shr<RHS>,
        RHS: Copy,
    {
        Vector3 {
            x: self.x >> rhs,
            y: self.y >> rhs,
            z: self.z >> rhs,
        }
    }

    #[inline(always)]
    pub fn shl<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        T: Shl<RHS>,
        RHS: Copy,
    {
        Vector3 {
            x: self.x << rhs,
            y: self.y << rhs,
            z: self.z << rhs,
        }
    }

    #[inline(always)]
    pub fn bit_and<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        T: BitAnd<RHS>,
        RHS: Copy,
    {
        Vector3 {
            x: self.x & rhs,
            y: self.y & rhs,
            z: self.z & rhs,
        }
    }

    #[inline(always)]
    pub fn bit_or<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        T: BitOr<RHS>,
        RHS: Copy,
    {
        Vector3 {
            x: self.x | rhs,
            y: self.y | rhs,
            z: self.z | rhs,
        }
    }

    #[inline(always)]
    pub fn bit_xor<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        T: BitXor<RHS>,
        RHS: Copy,
    {
        Vector3 {
            x: self.x ^ rhs,
            y: self.y ^ rhs,
            z: self.z ^ rhs,
        }
    }

    #[inline(always)]
    pub fn not(self) -> Vector3<T::Output>
    where
        T: Not,
    {
        Vector3 {
            x: !self.x,
            y: !self.y,
            z: !self.z,
        }
    }
}
