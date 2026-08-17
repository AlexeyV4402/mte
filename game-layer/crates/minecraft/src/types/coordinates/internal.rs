use super::core::{InternalCoords, InternalCoordsType};
use crate::types::chunk::CHUNK_SIZE_WITH_PADDING;
use crate::types::coordinates::core::LocalCoords;

impl InternalCoords {
    pub const STRIDE_Z: usize = CHUNK_SIZE_WITH_PADDING as usize;
    pub const STRIDE_Y: usize = Self::STRIDE_Z * Self::STRIDE_Z;
}

impl From<InternalCoords> for usize {
    #[inline]
    fn from(coords: InternalCoords) -> Self {
        let (x, y, z) = coords.into();

        (y as usize * InternalCoords::STRIDE_Y)
            + (z as usize * InternalCoords::STRIDE_Z)
            + x as usize
    }
}

impl From<usize> for InternalCoords {
    #[inline]
    fn from(id: usize) -> Self {
        let y = id / Self::STRIDE_Y;
        let rem = id % Self::STRIDE_Y;
        let z = rem / Self::STRIDE_Z;
        let x = rem % Self::STRIDE_Z;

        (
            x as InternalCoordsType,
            y as InternalCoordsType,
            z as InternalCoordsType,
        )
            .into()
    }
}

impl From<LocalCoords> for InternalCoords {
    #[inline]
    fn from(local: LocalCoords) -> Self {
        let (x, y, z) = local.into();
        (x + 1, y + 1, z + 1).into()
    }
}

impl TryFrom<InternalCoords> for LocalCoords {
    type Error = &'static str;

    #[inline]
    fn try_from(internal: InternalCoords) -> Result<Self, Self::Error> {
        let (x, y, z) = internal.into();

        // Если координата лежит на границе (0 или 33) — это воротник соседа,
        // у него нет локального адреса в этом чанке.
        if x == 0 || x == 33 || y == 0 || y == 33 || z == 0 || z == 33 {
            return Err("Попытка получить локальную координату для блока воротника!");
        }

        // Если проверка прошла, значит мы внутри игрового чанка (1..32).
        // Возвращаем честные 0..31, делая -1.
        Ok((x - 1, y - 1, z - 1).into())
    }
}
