use ash::vk::{self, Framebuffer, RenderPass};

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::DepthBuffer;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::swapchain_object::SwapchainObject;

pub struct FramebufferObject {
    pub framebuffers: Vec<Framebuffer>,
}

impl FramebufferObject {
    pub fn new(
        device: &ash::Device,
        swapchain_object: &SwapchainObject,
        depth_buffer: &DepthBuffer,
        render_pass: RenderPass,
    ) -> Self {
        let mut framebuffers = Vec::new();
        Self::fill_buffers(
            &mut framebuffers,
            device,
            swapchain_object,
            depth_buffer,
            render_pass,
        );
        Self { framebuffers }
    }
    #[inline]
    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            for &fb in &self.framebuffers {
                device.destroy_framebuffer(fb, None);
            }
        }
        self.framebuffers.clear();
    }

    #[inline]
    pub fn recreate(
        &mut self,
        device: &ash::Device,
        swapchain_object: &SwapchainObject,
        depth_buffer: &DepthBuffer,
        render_pass: RenderPass,
    ) {
        Self::fill_buffers(
            &mut self.framebuffers,
            device,
            swapchain_object,
            depth_buffer,
            render_pass,
        );
    }

    #[inline]
    fn fill_buffers(
        framebuffers: &mut Vec<Framebuffer>,
        device: &ash::Device,
        swapchain_object: &SwapchainObject,
        depth_buffer: &DepthBuffer,
        render_pass: RenderPass,
    ) {
        for &swapchain_view in &swapchain_object.image_views {
            let attachments = [
                swapchain_view,    // Порядок СТРОГО как в RenderPass (Цвет — 0)
                depth_buffer.view, // Глубина — 1
            ];

            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass) // Наш RenderPass пересоздавать НЕ НАДО, его схема не изменилась
                .attachments(&attachments)
                .width(swapchain_object.extent.width)
                .height(swapchain_object.extent.height)
                .layers(1);

            let framebuffer = unsafe {
                device
                    .create_framebuffer(&framebuffer_info, None)
                    .expect("Не удалось воссоздать Framebuffer при ресайзе")
            };

            framebuffers.push(framebuffer);
        }
    }
}
