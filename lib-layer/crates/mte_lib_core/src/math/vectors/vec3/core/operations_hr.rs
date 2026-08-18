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
    pub fn all_gt<RHS>(self, other: RHS) -> bool
    where
        T: PartialOrd<RHS>,
    {
        self.x > other && self.y > other && self.z > other
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
