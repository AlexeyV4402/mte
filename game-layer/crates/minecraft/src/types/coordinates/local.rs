use super::core::{LocalCoords, LocalCoordsType};
use crate::types::chunk::{CHUNK_SIZE, CHUNK_SIZE_WITH_PADDING};

impl LocalCoords {
    const STRIDE_Z: usize = CHUNK_SIZE_WITH_PADDING as usize;
    const STRIDE_Y: usize = Self::STRIDE_Z * Self::STRIDE_Z;
    pub fn in_chunk(self) -> bool {
        self.0.all_lt(CHUNK_SIZE)
    }
}

// В массиве чанка сначала идут координаты вдоль X, потом они складываются в ряды вдоль Z, затем вдоль Y
// Допустим, имеем массив 4x4x4.
// 0000000000000000000000000000000000000000000000000000000000000000
// ^
// X
// ^^^^
//    Z
// ^^^^^^^^^^^^^^^^
//                Y

impl From<LocalCoords> for usize {
    #[inline]
    fn from(coords: LocalCoords) -> Self {
        let (x, y, z) = coords.into();

        let internal_x = x as usize + 1;
        let internal_y = y as usize + 1;
        let internal_z = z as usize + 1;

        (internal_y * LocalCoords::STRIDE_Y) + (internal_z * LocalCoords::STRIDE_Z) + internal_x
    }
}

impl From<usize> for LocalCoords {
    #[inline]
    fn from(id: usize) -> Self {
        let internal_y = id / Self::STRIDE_Y;
        let rem = id % Self::STRIDE_Y;
        let internal_z = rem / Self::STRIDE_Z;
        let internal_x = rem % Self::STRIDE_Z;

        let x = (internal_x as isize - 1) as LocalCoordsType;
        let y = (internal_y as isize - 1) as LocalCoordsType;
        let z = (internal_z as isize - 1) as LocalCoordsType;

        (x, y, z).into()
    }
}
