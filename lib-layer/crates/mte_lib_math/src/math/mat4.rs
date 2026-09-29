use glam::Mat4;

pub trait SafeInvert {
    fn safe_inverse(&self) -> Mat4;
}

impl SafeInvert for Mat4 {
    fn safe_inverse(&self) -> Mat4 {
        let inv = self.inverse();
        if inv.is_finite() { inv } else { Mat4::IDENTITY }
    }
}
