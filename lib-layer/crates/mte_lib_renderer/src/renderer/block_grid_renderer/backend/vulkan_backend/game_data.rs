use ash::vk;
use ash::vk::*;
use glam::Mat4;
use mte_macros::vfs_include_vk_shader;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::VkBuilder;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::renderer::{
    RenderData, VkBackend
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataHV;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::descriptors::Descriptors;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::game_buffer_manager::{
    ChunkGpuHandle, GameBufferManager, HandGpuHandle, PhysObjectGpuHandle
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::static_data::StaticData;
use crate::renderer::block_grid_renderer::render_objects::camera::WorldCameraUniform;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use crate::renderer::block_grid_renderer::types::RendererCreateArgs;

pub struct VkInGameData {
    pub chunks_pipeline_layout: vk::PipelineLayout,
    pub chunks_pipeline: vk::Pipeline,

    pub hand_pipeline_layout: vk::PipelineLayout,
    pub hand_pipeline: vk::Pipeline,

    pub phys_objects_pipeline_layout: vk::PipelineLayout,
    pub phys_objects_pipeline: vk::Pipeline,

    // --- НАШ МЕНЕДЖЕР БУФЕРОВ ---
    pub buffer_manager: GameBufferManager,

    // Буфер для камеры на GPU (HOST_VISIBLE | HOST_COHERENT) и замаппленный указатель на него
    pub camera_buffer: VkBufferDataHV,

    pub descriptors: Descriptors,

    pub static_data: StaticData,

    pub hand_mat: [[f32; 4]; 4],
}

impl VkInGameData {
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        render_pass: vk::RenderPass,
        graphics_queue: vk::Queue,
        cmd_pool: vk::CommandPool,
        data_create_args: RendererCreateArgs,
    ) -> Self {
        let chunks_vert = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/chunks.vertex.glsl"
        );
        let chunks_frag = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/chunks.fragment.glsl"
        );
        let hand_vert = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/hand.vertex.glsl"
        );
        let hand_frag = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/hand.fragment.glsl"
        );
        let phys_objects_vert = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/phys_objects.vertex.glsl"
        );
        let phys_objects_frag = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/phys_objects.fragment.glsl"
        );

        let chunks_vert_shader_module = VkBuilder::create_shader_module(device, chunks_vert);
        let chunks_frag_shader_module = VkBuilder::create_shader_module(device, chunks_frag);
        let hand_vert_shader_module = VkBuilder::create_shader_module(device, hand_vert);
        let hand_frag_shader_module = VkBuilder::create_shader_module(device, hand_frag);
        let phys_objects_vert_shader_module =
            VkBuilder::create_shader_module(device, phys_objects_vert);
        let phys_objects_frag_shader_module =
            VkBuilder::create_shader_module(device, phys_objects_frag);

        let mut buffer_manager = GameBufferManager::new(device, mem_properties);

        let camera_buffer = VkBufferDataHV::new(
            device,
            &mem_properties,
            size_of::<WorldCameraUniform>() as u64,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );

        let mut static_data = StaticData::new(
            device,
            &mem_properties,
            data_create_args.block_properties.len(),
            data_create_args.layer_count,
        );

        let descriptors = Descriptors::new(
            device,
            &static_data,
            camera_buffer.buffer,
            buffer_manager.vector_buffer_manager.buffers[0]
                .buffer
                .buffer,
            buffer_manager.matrix_buffer_manager.buffers[0]
                .buffer
                .buffer,
        );

        let chunks_pipeline_layout =
            VkBuilder::create_pipeline_layout(device, &descriptors.chunks_layouts());

        let chunks_pipeline = VkBuilder::create_graphics_pipeline(
            device,
            chunks_pipeline_layout,
            render_pass,
            chunks_vert_shader_module,
            chunks_frag_shader_module,
        );

        let phys_objects_pipeline_layout =
            VkBuilder::create_pipeline_layout(device, &descriptors.phys_objects_layouts());

        let phys_objects_pipeline = VkBuilder::create_graphics_pipeline(
            device,
            phys_objects_pipeline_layout,
            render_pass,
            phys_objects_vert_shader_module,
            phys_objects_frag_shader_module,
        );

        let hand_pipeline_layout = VkBuilder::create_pipeline_layout_with_push_const_range(
            device,
            &descriptors.hand_layouts(),
            64,
        );

        let hand_pipeline = VkBuilder::create_graphics_pipeline(
            device,
            hand_pipeline_layout,
            render_pass,
            hand_vert_shader_module,
            hand_frag_shader_module,
        );

        unsafe {
            static_data.upload(
                device,
                graphics_queue,
                cmd_pool,
                &mut buffer_manager,
                data_create_args.block_properties,
                16,
                data_create_args.layer_count,
            );
        };

        Self {
            chunks_pipeline_layout,
            chunks_pipeline,
            hand_pipeline_layout,
            hand_pipeline,
            phys_objects_pipeline_layout,
            phys_objects_pipeline,
            buffer_manager,
            camera_buffer,
            descriptors,
            static_data,
            hand_mat: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn load_chunk(
        &mut self,
        renderer: &VkBackend,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
    ) -> Option<ChunkGpuHandle> {
        let device = &renderer.device;
        let mem_properties = &renderer.memory_prop;
        let current_cmd = renderer.get_current_cmd();

        let handle = self.buffer_manager.load_chunk(
            primitive,
            vector,
            device,
            mem_properties,
            current_cmd,
            &self.descriptors,
            self.descriptors.set2_vectors,
        );

        handle
    }

    pub fn load_phys_object(
        &mut self,
        renderer: &VkBackend,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
        matrix: [[f32; 4]; 4],
    ) -> Option<PhysObjectGpuHandle> {
        let device = &renderer.device;
        let mem_properties = &renderer.memory_prop;
        let current_cmd = renderer.get_current_cmd();

        let handle = self.buffer_manager.load_phys_object(
            primitive,
            vector,
            matrix,
            device,
            mem_properties,
            current_cmd,
            &self.descriptors,
            self.descriptors.set2_vectors,
            self.descriptors.set3_matrices,
        );

        handle
    }

    pub fn update_phys_object(
        &mut self,
        renderer: &VkBackend,
        handle: PhysObjectGpuHandle,
        vector: [i32; 4],
        matrix: [[f32; 4]; 4],
    ) {
        let device = &renderer.device;
        let current_cmd = renderer.get_current_cmd();
        self.buffer_manager
            .update_phys_object(handle, vector, matrix, device, current_cmd);
    }

    pub fn unload_phys_object(&mut self, handle: PhysObjectGpuHandle) {
        self.buffer_manager.unload_phys_object(handle);
    }

    pub fn load_hand(
        &mut self,
        renderer: &VkBackend,
        primitive: BlockIndexedPrimitive,
        matrix: [[f32; 4]; 4],
    ) -> Option<HandGpuHandle> {
        let device = &renderer.device;
        let mem_properties = &renderer.memory_prop;
        let current_cmd = renderer.get_current_cmd();

        self.hand_mat = matrix;
        let handle = self
            .buffer_manager
            .load_hand(primitive, device, mem_properties, current_cmd);

        handle
    }

    pub fn unload_chunk(&mut self, handle: ChunkGpuHandle) {
        self.buffer_manager.unload_chunk(handle);
    }

    pub fn unload_hand(&mut self, data: HandGpuHandle) {
        self.hand_mat = Mat4::IDENTITY.to_cols_array_2d();
        self.buffer_manager.unload_hand(data);
    }

    pub fn update_camera(&mut self, pass_1_camera_uniform: WorldCameraUniform) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                &pass_1_camera_uniform as *const WorldCameraUniform,
                self.camera_buffer.mapped_ptr as *mut WorldCameraUniform,
                1,
            );
        }
    }
}

impl RenderData for VkInGameData {
    fn prepare_buffers(&mut self, renderer: &VkBackend) {
        self.buffer_manager
            .prepare_buffers(&renderer.device, renderer.get_current_cmd());
    }

    fn draw(&self, renderer: &VkBackend) {
        let device = &renderer.device;
        let current_cmd = renderer.get_current_cmd();
        let swapchain_extent = renderer.swapchain_object.extent;

        unsafe {
            // ====================================================================
            // ШАГ 4: ВКЛЮЧАЕМ КОНВЕЙЕР И ДЕСКРИПТОРЫ
            // ====================================================================

            device.cmd_bind_pipeline(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.chunks_pipeline,
            );

            device.cmd_bind_descriptor_sets(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.chunks_pipeline_layout,
                0,
                &self.descriptors.chunks_sets(),
                &[],
            );

            // ====================================================================
            // ШАГ 5: МУЛЬТИ-ДРОУ ВЫЗОВ (ОТРИСОВКА МИРА)
            // ====================================================================
            let stride = size_of::<DrawIndexedIndirectCommand>() as u32;
            self.buffer_manager
                .chunk_render_buffer_manager
                .buffers
                .iter()
                .for_each(|buffer| {
                    device.cmd_bind_vertex_buffers(
                        current_cmd,
                        0,
                        &[buffer.vertex_buffer.buffer],
                        &[0],
                    );

                    device.cmd_bind_index_buffer(
                        current_cmd,
                        buffer.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );

                    device.cmd_draw_indexed_indirect(
                        current_cmd,
                        buffer.indirect_buffer.buffer,
                        0,
                        buffer.cpu_indirect_buffer.len() as u32,
                        stride,
                    );
                });

            device.cmd_bind_pipeline(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.phys_objects_pipeline,
            );

            device.cmd_bind_descriptor_sets(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.phys_objects_pipeline_layout,
                0,
                &self.descriptors.phys_objects_sets(),
                &[],
            );

            self.buffer_manager
                .phys_render_buffer_manager
                .buffers
                .iter()
                .for_each(|buffer| {
                    device.cmd_bind_vertex_buffers(
                        current_cmd,
                        0,
                        &[buffer.vertex_buffer.buffer],
                        &[0],
                    );

                    device.cmd_bind_index_buffer(
                        current_cmd,
                        buffer.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );

                    device.cmd_draw_indexed_indirect(
                        current_cmd,
                        buffer.indirect_buffer.buffer,
                        0,
                        buffer.cpu_indirect_buffer.len() as u32,
                        stride,
                    );
                });

            let clear_attachment = vk::ClearAttachment::default()
                .aspect_mask(vk::ImageAspectFlags::DEPTH) // Стираем только карту глубины кадра
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                });

            let clear_rect = vk::ClearRect::default()
                .rect(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain_extent,
                })
                .layer_count(1);

            device.cmd_clear_attachments(current_cmd, &[clear_attachment], &[clear_rect]);

            // Включаем конвейер руки
            device.cmd_bind_pipeline(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.hand_pipeline,
            );

            device.cmd_bind_descriptor_sets(
                current_cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.hand_pipeline_layout,
                0,
                &self.descriptors.hand_sets(),
                &[],
            );

            // ТОЛКАЕМ МАТРИЦУ РУКИ НА GPU (Push-константы)
            let hand_bytes = bytemuck::cast_slice(&self.hand_mat);
            device.cmd_push_constants(
                current_cmd,
                self.hand_pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                &hand_bytes,
            );

            // Вызываем INDIRECT-отрисовку руки из САМОГО НАЧАЛА буфера (Слот №0)
            self.buffer_manager
                .hand_render_buffer_manager
                .buffers
                .iter()
                .for_each(|buffer| {
                    device.cmd_bind_vertex_buffers(
                        current_cmd,
                        0,
                        &[buffer.vertex_buffer.buffer],
                        &[0],
                    );

                    device.cmd_bind_index_buffer(
                        current_cmd,
                        buffer.index_buffer.buffer,
                        0,
                        vk::IndexType::UINT32,
                    );

                    device.cmd_draw_indexed_indirect(
                        current_cmd,
                        buffer.indirect_buffer.buffer,
                        0,
                        buffer.cpu_indirect_buffer.len() as u32,
                        stride,
                    );
                });
        }
    }
}
