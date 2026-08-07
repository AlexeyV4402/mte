use std::sync::Arc;

use winit::window::Window;

use super::gpu_context::GpuContext;
use super::window_context::WindowContext;

pub struct RenderContext {
    pub gpu_contexts: [GpuContext; 1],
    pub window_contexts: [WindowContext; 1],
}

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<RenderContext> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                // Активируем необходимые расширения
                required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                    | wgpu::Features::INDIRECT_FIRST_INSTANCE,
                required_limits: wgpu::Limits {
                    // Увеличиваем лимит на количество текстур в одном BindGroup
                    max_bindings_per_bind_group: 1024,
                    ..wgpu::Limits::default()
                },
                ..Default::default()
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            // present_mode: surface_caps.present_modes[0],
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            // NEW!
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        Ok(Self {
            gpu_contexts: [GpuContext { device, queue }],
            window_contexts: [WindowContext {
                window,
                surface,
                config,
                is_surface_configured: false,
            }],
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_contexts[0].resize(&self.gpu_contexts[0].device, width, height);
    }
}
