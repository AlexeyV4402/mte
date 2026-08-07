use crate::coordinates::{
    ChunkCoords, ChunkCoordsType, GlobalCoords, GlobalCoordsType, LocalCoords, LocalCoordsType
};

impl GlobalCoords {
    pub fn get_chunk(self) -> ChunkCoords {
        ChunkCoords {
            x: (self.x >> 5) as ChunkCoordsType,
            y: (self.y >> 5) as ChunkCoordsType,
            z: (self.z >> 5) as ChunkCoordsType,
        }
    }

    pub fn get_local(self) -> LocalCoords {
        LocalCoords {
            x: (self.x & 31) as LocalCoordsType,
            y: (self.y & 31) as LocalCoordsType,
            z: (self.z & 31) as LocalCoordsType,
        }
    }
}

impl From<(ChunkCoords, LocalCoords)> for GlobalCoords {
    fn from(data: (ChunkCoords, LocalCoords)) -> Self {
        Self {
            x: ((data.0.x << 5) + (data.1.x as ChunkCoordsType)) as GlobalCoordsType,
            y: ((data.0.y << 5) + (data.1.y as ChunkCoordsType)) as GlobalCoordsType,
            z: ((data.0.z << 5) + (data.1.z as ChunkCoordsType)) as GlobalCoordsType,
        }
    }
}
