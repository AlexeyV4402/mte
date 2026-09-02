use glam::{Mat4, Vec3};

use super::core::ChunkCoords;
use crate::types::chunk::CHUNK_SIZE;
use crate::types::coordinates::core::{Coords, InRegionCoords, InRegionCoordsType, RegionCoords};

impl ChunkCoords {
    pub fn get_model_mat(self) -> [[f32; 4]; 4] {
        Mat4::from_translation(Vec3::from((self.0 * CHUNK_SIZE as i32).as_f32())).to_cols_array_2d()
    }

    pub fn get_region(self) -> RegionCoords {
        Coords::from(self.0.shr_all(5))
    }

    pub fn in_region(self) -> InRegionCoords {
        Coords::from(self.0.bit_and_all(0x1F))
    }
}

impl From<InRegionCoords> for usize {
    #[inline]
    fn from(coords: InRegionCoords) -> Self {
        let (x, y, z) = coords.0.as_usize().into();
        x + (z << 5) + (y << 10)
    }
}

impl From<usize> for InRegionCoords {
    #[inline]
    fn from(id: usize) -> Self {
        let y = id / 1024;
        let rem = id % 1024;
        let z = rem / 32;
        let x = rem % 32;

        (
            x as InRegionCoordsType,
            y as InRegionCoordsType,
            z as InRegionCoordsType,
        )
            .into()
    }
}
