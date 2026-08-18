use super::core::Vector3;

impl From<Vector3<f32>> for glam::Vec3 {
    #[inline(always)]
    fn from(v: Vector3<f32>) -> Self {
        glam::Vec3::new(v.x, v.y, v.z)
    }
}

impl From<Vector3<i32>> for glam::IVec3 {
    #[inline(always)]
    fn from(v: Vector3<i32>) -> Self {
        glam::IVec3::new(v.x, v.y, v.z)
    }
}

impl Into<Vector3<f32>> for glam::Vec3 {
    #[inline(always)]
    fn into(self) -> Vector3<f32> {
        Vector3::new(self.x, self.y, self.z)
    }
}

impl Into<Vector3<i32>> for glam::IVec3 {
    #[inline(always)]
    fn into(self) -> Vector3<i32> {
        Vector3::new(self.x, self.y, self.z)
    }
}
