use ash::vk;
use ash::vk::*;
use mte_macros::vfs_include_vk_shader;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::VkBuilder;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::renderer::{
    RenderData, VkBackend
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::gui_buffer_manager::GuiBufferManager;
use crate::renderer::block_grid_renderer::render_objects::primitive::GuiIndexedPrimitive;

pub struct VkGuiData {
    pub standart_pipeline_layout: vk::PipelineLayout,
    pub standart_pipeline: vk::Pipeline,

    pub buffer_manager: GuiBufferManager,
}

impl VkGuiData {
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        render_pass: vk::RenderPass,
    ) -> Self {
        let standart_vert = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/gui_standart.vertex.glsl"
        );
        let standart_frag = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/gui_standart.fragment.glsl"
        );

        let standart_vert_shader_module = VkBuilder::create_shader_module(device, standart_vert);
        let standart_frag_shader_module = VkBuilder::create_shader_module(device, standart_frag);

        let buffer_manager = GuiBufferManager::new(device, mem_properties);

        let standart_pipeline_layout = VkBuilder::create_pipeline_layout(device, &[]);

        let standart_pipeline = VkBuilder::create_graphics_gui_pipeline(
            device,
            standart_pipeline_layout,
            render_pass,
            standart_vert_shader_module,
            standart_frag_shader_module,
        );

        Self {
            standart_pipeline_layout,
            standart_pipeline,
            buffer_manager,
        }
    }

    pub fn load_quad(&mut self, renderer: &VkBackend, primitive: GuiIndexedPrimitive, idx: u64) {
        self.buffer_manager
            .load_quad(primitive, idx, &renderer.device, renderer.get_current_cmd());
    }

    pub fn destroy(self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline_layout(self.standart_pipeline_layout, None);
            device.destroy_pipeline(self.standart_pipeline, None);
            self.buffer_manager.destroy(device);
        }
    }
}

impl RenderData for VkGuiData {
    fn prepare_buffers(&mut self, renderer: &VkBackend) {
        self.buffer_manager
            .prepare_buffers(&renderer.device, renderer.get_current_cmd());
    }

    fn draw(&self, renderer: &VkBackend) {
        let device = &renderer.device;
        let current_cmd = renderer.get_current_cmd();
        unsafe {
            // ====================================================================
            // ШАГ 4: ВКЛЮЧАЕМ КОНВЕЙЕР И ДЕСКРИПТОРЫ
            // ====================================================================

            device.cmd_bind_pipeline(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.standart_pipeline,
            );

            // ====================================================================
            // ШАГ 5: МУЛЬТИ-ДРОУ ВЫЗОВ
            // ====================================================================
            let stride = size_of::<DrawIndexedIndirectCommand>() as u32;

            device.cmd_bind_vertex_buffers(
                current_cmd,
                0,
                &[self.buffer_manager.vertex_buffer.buffer],
                &[0],
            );

            device.cmd_bind_index_buffer(
                current_cmd,
                self.buffer_manager.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );

            device.cmd_draw_indexed_indirect(
                current_cmd,
                self.buffer_manager.gpu_indirect_buffer.buffer,
                0,
                self.buffer_manager.cpu_indirect_buffer.len() as u32,
                stride,
            );
        }
    }
}
