use crate::chunk::CHUNK_SIZE;
use crate::coordinates::{LocalCoords, LocalCoordsType};

impl LocalCoords {
    pub fn in_chunk(self) -> bool {
        self.x < CHUNK_SIZE && self.y < CHUNK_SIZE && self.z < CHUNK_SIZE
    }
}

impl From<LocalCoords> for usize {
    fn from(coords: LocalCoords) -> Self {
        let (x, y, z) = coords.into();
        ((y << 10) | (z << 5) | x) as usize
    }
}

impl From<usize> for LocalCoords {
    fn from(id: usize) -> Self {
        let x = id & 31; // Первые 5 бит (0..4)
        let z = (id >> 5) & 31; // Следующие 5 бит (5..9)
        let y = (id >> 10) & 31; // Последние 5 бит (10..14)
        (
            x as LocalCoordsType,
            y as LocalCoordsType,
            z as LocalCoordsType,
        )
            .into()
    }
}
