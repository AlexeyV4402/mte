use crate::types::coordinates::core::{MiniChunkCoords, MiniChunkCoordsType};

impl From<MiniChunkCoords> for usize {
    #[inline]
    fn from(coords: MiniChunkCoords) -> Self {
        let (x, y, z) = coords.into();

        (((y) << 6) | ((z) << 3) | (x)) as usize
    }
}

impl From<usize> for MiniChunkCoords {
    #[inline]
    fn from(id: usize) -> Self {
        let x = id & 0x7;
        let y = id >> 6;
        let z = (id >> 3) & 0x7;

        (
            x as MiniChunkCoordsType,
            y as MiniChunkCoordsType,
            z as MiniChunkCoordsType,
        )
            .into()
    }
}
