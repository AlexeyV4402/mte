use crate::math::vectors::vec3::core::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrecisePosition<const CHUNK_SIZE: usize> {
    pub chunk: Vector3<i32>,
    pub in_chunk: Vector3<f32>,
}

impl<const CHUNK_SIZE: usize> PrecisePosition<CHUNK_SIZE> {
    const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
    const ZERO: Self = Self {
        chunk: Vector3::new(0, 0, 0),
        in_chunk: Vector3::new(0.0, 0.0, 0.0),
    };

    #[inline(always)]
    pub fn normalize(&mut self) {
        let chunk = (self.in_chunk / Self::CHUNK_SIZE_F32).floor_cw();

        self.chunk += chunk.as_i32();

        self.in_chunk -= chunk * Self::CHUNK_SIZE_F32;
    }

    #[inline]
    pub fn raw_new(chunk: Vector3<i32>, in_chunk: Vector3<f32>) -> Self {
        Self { chunk, in_chunk }
    }

    #[inline(always)]
    pub fn normalized(&self) -> Self {
        let chunk = (self.in_chunk / Self::CHUNK_SIZE_F32).floor_cw();
        Self {
            chunk: self.chunk + chunk.as_i32(),
            in_chunk: self.in_chunk - chunk * Self::CHUNK_SIZE_F32,
        }
    }

    #[inline(always)]
    pub fn raw_add(self, rhs: PrecisePosition<CHUNK_SIZE>) -> PrecisePosition<CHUNK_SIZE> {
        PrecisePosition {
            chunk: self.chunk + rhs.chunk,
            in_chunk: self.in_chunk + rhs.in_chunk,
        }
    }
}

pub type PrecisePositionC32 = PrecisePosition<32>;
