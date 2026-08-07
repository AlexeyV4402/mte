use std::sync::Arc;

use winit::window::Window;

pub struct WindowContext {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
}

impl WindowContext {
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.is_surface_configured = true;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }
}
