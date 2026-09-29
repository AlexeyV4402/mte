use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Rem, Shl, Shr};

use super::Vector3;

/// Component-wise methods
impl<T> Vector3<T> {
    #[inline(always)]
    pub fn shr_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
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
    pub fn shl_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
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
    pub fn bit_and_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
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
    pub fn bit_or_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
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
    pub fn bit_xor_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
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
    pub fn add_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        RHS: Copy,
        T: Add<RHS>,
    {
        Vector3 {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }

    #[inline(always)]
    pub fn rem_all<RHS>(self, rhs: RHS) -> Vector3<T::Output>
    where
        RHS: Copy,
        T: Rem<RHS>,
    {
        Vector3 {
            x: self.x % rhs,
            y: self.y % rhs,
            z: self.z % rhs,
        }
    }
}
