#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Air,
    Dirt,
    Stone,
}

impl Block {
    #[inline]
    pub fn is_solid(self) -> bool {
        self != Self::Air
    }
}
