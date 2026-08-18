use super::core::Vector3;

pub type Vec3u8 = Vector3<u8>;
pub type Vec3u16 = Vector3<u16>;
pub type Vec3u32 = Vector3<u32>;
pub type Vec3u64 = Vector3<u64>;
pub type Vec3u128 = Vector3<u128>;
pub type Vec3usize = Vector3<usize>;

pub type Vec3i8 = Vector3<i8>;
pub type Vec3i16 = Vector3<i16>;
pub type Vec3i32 = Vector3<i32>;
pub type Vec3i64 = Vector3<i64>;
pub type Vec3i128 = Vector3<i128>;
pub type Vec3isize = Vector3<isize>;

// pub type Vec3f16 = Vector3<f16>;
pub type Vec3f32 = Vector3<f32>;
pub type Vec3f64 = Vector3<f64>;
// pub type Vec3f128 = Vector3<f128>;

impl Vector3<f32> {
    #[inline(always)]
    pub fn floor_cw(self) -> Self {
        Self::new(self.x.floor(), self.y.floor(), self.z.floor())
    }
}

#[macro_export]
macro_rules! impl_vector_cast {
    // Шаблон: ИсходныйТип => ЦелевойТип, имя_метода
    ($from_type:ty => $to_type:ty, $method_name:ident) => {
        impl Vector3<$from_type> {
            /// Быстрое приведение типов для всех осей вектора за 0 тактов CPU.
            #[inline(always)]
            pub fn $method_name(self) -> Vector3<$to_type> {
                Vector3 {
                    x: self.x as $to_type,
                    y: self.y as $to_type,
                    z: self.z as $to_type,
                }
            }
        }
    };
}

// .as_u32()
impl_vector_cast!(i32 => u32, as_u32);
impl_vector_cast!(i64 => u32, as_u32);

// .as_usize()
impl_vector_cast!(i32 => usize, as_usize);
impl_vector_cast!(i64 => usize, as_usize);

// .as_i32()
impl_vector_cast!(f32 => i32, as_i32);
impl_vector_cast!(u32 => i32, as_i32);
impl_vector_cast!(i64 => i32, as_i32);

// .as_f32()
impl_vector_cast!(i32 => f32, as_f32);

// .as_i64()
impl_vector_cast!(i32 => i64, as_i64);
impl_vector_cast!(f32 => i64, as_i64);

// impl Vector3<i32> {
//     #[inline(always)]
//     pub fn simd_add(self, rhs: Self) -> Self {
//         #[cfg(target_arch = "x86_64")]
//         use std::arch::x86_64::*;

//         unsafe {
//             // Упаковываем наши i32 в 128-битный SSE регистр (влезает 4 штуки i32)
//             let a = _mm_set_epi32(0, self.z, self.y, self.x);
//             let b = _mm_set_epi32(0, rhs.z, rhs.y, rhs.x);
//             // Складываем ВСЕ оси за 1 такт процессора на аппаратном уровне!
//             let res = _mm_add_epi32(a, b);

//             // Выгружаем обратно
//             let mut out = [0i32; 4];
//             _mm_storeu_si128(out.as_mut_ptr() as *mut __m128i, res);

//             Vector3 { x: out[0], y: out[1], z: out[2] }
//         }
//     }
// }
