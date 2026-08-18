use super::core::{ChunkCoords, GlobalCoords, LocalCoords};

impl GlobalCoords {
    pub fn get_chunk(self) -> ChunkCoords {
        ChunkCoords::from(self.0.as_i32().shr(5))
    }

    pub fn get_local(self) -> LocalCoords {
        LocalCoords::from(self.0.bit_and(31).as_u32())
    }
}

impl From<(ChunkCoords, LocalCoords)> for GlobalCoords {
    fn from(data: (ChunkCoords, LocalCoords)) -> Self {
        Self::from((data.0.0.shl(5) + data.1.0.as_i32()).as_i64())
    }
}
