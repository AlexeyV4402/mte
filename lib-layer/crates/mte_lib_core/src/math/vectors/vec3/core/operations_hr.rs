use std::ops::{Add, Mul};

use super::Vector3;

/// Horizontal reduction methods
impl<T> Vector3<T> {
    #[inline(always)]
    pub fn all_lt<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x < other && self.y < other && self.z < other
    }

    #[inline(always)]
    pub fn all_le<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x <= other && self.y <= other && self.z <= other
    }

    #[inline(always)]
    pub fn all_gt<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x > other && self.y > other && self.z > other
    }

    #[inline(always)]
    pub fn all_ge<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x >= other && self.y >= other && self.z >= other
    }

    #[inline(always)]
    pub fn any_lt<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x < other || self.y < other || self.z < other
    }

    #[inline(always)]
    pub fn any_gt<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x > other || self.y > other || self.z > other
    }

    #[inline(always)]
    pub fn all_lt_cw(self, rhs: Self) -> bool
    where
        T: PartialOrd<T>,
    {
        self.x < rhs.x && self.y < rhs.y && self.z < rhs.z
    }

    #[inline(always)]
    pub fn all_le_cw(self, rhs: Self) -> bool
    where
        T: PartialOrd<T>,
    {
        self.x <= rhs.x && self.y <= rhs.y && self.z <= rhs.z
    }

    #[inline(always)]
    pub fn all_gt_cw(self, rhs: Self) -> bool
    where
        T: PartialOrd<T>,
    {
        self.x > rhs.x && self.y > rhs.y && self.z > rhs.z
    }

    #[inline(always)]
    pub fn all_ge_cw(self, rhs: Self) -> bool
    where
        T: PartialOrd<T>,
    {
        self.x >= rhs.x && self.y >= rhs.y && self.z >= rhs.z
    }

    #[inline(always)]
    pub fn sum(self) -> T
    where
        T: Add<Output = T>,
    {
        self.x + self.y + self.z
    }

    #[inline(always)]
    pub fn product(self) -> T
    where
        T: Mul<Output = T>,
    {
        self.x * self.y * self.z
    }
}
