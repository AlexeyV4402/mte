use std::mem;

use ash::vk;
use ash::vk::*;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::debug::VulkanNameable;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_manager::{
    RenderAllocData, RenderBufferManager, SlotAllocData, SlotBufferManager
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::{
    VkBufferDataDL, VkBufferDataHV
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::descriptors::Descriptors;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use crate::renderer::block_grid_renderer::types::BlockVertex;

pub struct StagingBufferCommand {
    pub src_offset: u64,
    pub dst_offset: u64,
    pub length: u64,
    pub dst: vk::Buffer,
}

pub struct IndirectBufferManager {
    pub gpu_staging_buffer: VkBufferDataHV,
    pub cpu_staging_buffer: Vec<u8>,

    pub cpu_staging_queue: Vec<StagingBufferCommand>,

    pub chunk_render_buffer_manager: RenderBufferManager,
    pub phys_render_buffer_manager: RenderBufferManager,
    pub hand_render_buffer_manager: RenderBufferManager,
    pub matrix_buffer_manager: SlotBufferManager<
        { size_of::<[[f32; 4]; 4]>() },
        { Self::ONE_MATRIX_BUFFER_SLOTS_COUNT as usize },
    >,
    pub vector_buffer_manager: SlotBufferManager<
        { size_of::<[i32; 4]>() },
        { Self::ONE_VECTOR_BUFFER_SLOTS_COUNT as usize },
    >,
}

impl IndirectBufferManager {
    pub const ONE_VERTEX_BUFFER_PAGE_CAPACITY: u64 = 4 * 1024;
    pub const ONE_VERTEX_BUFFER_PAGE_COUNT: u64 = 64 * 256;
    pub const ONE_VERTEX_BUFFER_CAPACITY: u64 =
        Self::ONE_VERTEX_BUFFER_PAGE_CAPACITY * Self::ONE_VERTEX_BUFFER_PAGE_COUNT;

    pub const ONE_INDEX_BUFFER_PAGE_CAPACITY: u64 = 4 * 1024;
    pub const ONE_INDEX_BUFFER_PAGE_COUNT: u64 = 64 * 256;
    pub const ONE_INDEX_BUFFER_CAPACITY: u64 =
        Self::ONE_INDEX_BUFFER_PAGE_CAPACITY * Self::ONE_INDEX_BUFFER_PAGE_COUNT;

    pub const ONE_MATRIX_BUFFER_SLOTS_COUNT: u64 = 256;
    pub const ONE_MATRIX_BUFFER_CAPACITY: u64 =
        Self::ONE_MATRIX_BUFFER_SLOTS_COUNT * (size_of::<[[f32; 4]; 4]>() as u64);

    pub const ONE_VECTOR_BUFFER_SLOTS_COUNT: u64 = 256;
    pub const ONE_VECTOR_BUFFER_CAPACITY: u64 =
        Self::ONE_VECTOR_BUFFER_SLOTS_COUNT * (size_of::<[i32; 4]>() as u64);

    pub const STAGING_BUFFER_CAPACITY: u64 = 128 * 1024 * 1024;

    pub const INDIRECT_BUFFER_SLOTS_COUNT: u64 =
        Self::ONE_VECTOR_BUFFER_SLOTS_COUNT + Self::ONE_MATRIX_BUFFER_SLOTS_COUNT + 1;

    pub const ONE_INDIRECT_BUFFER_SLOTS_COUNT: u64 = 128;
    pub const ONE_INDIRECT_BUFFER_CAPACITY: u64 =
        128 * (size_of::<DrawIndexedIndirectCommand>() as u64);

    pub const INDIRECT_BUFFER_CAPACITY: u64 =
        Self::INDIRECT_BUFFER_SLOTS_COUNT * (size_of::<DrawIndexedIndirectCommand>() as u64);

    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        let a = Self {
            gpu_staging_buffer: VkBufferDataHV::new(
                device,
                mem_properties,
                Self::STAGING_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_SRC,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            ),
            cpu_staging_buffer: Vec::with_capacity(Self::STAGING_BUFFER_CAPACITY as usize),
            cpu_staging_queue: Vec::new(),
            chunk_render_buffer_manager: RenderBufferManager::new(device, mem_properties),
            phys_render_buffer_manager: RenderBufferManager::new(device, mem_properties),
            hand_render_buffer_manager: RenderBufferManager::new(device, mem_properties),
            matrix_buffer_manager: SlotBufferManager::new(device, mem_properties),
            vector_buffer_manager: SlotBufferManager::new(device, mem_properties),
        };
        // a.index_buffer.gpu_buffers[0]
        //     .buffer
        //     .set_name("Index Buffer");
        // a.vertex_buffer.gpu_buffers[0]
        //     .buffer
        //     .set_name("Vertex Buffer");
        // a.vector_buffer.gpu_buffers[0]
        //     .buffer
        //     .set_name("Vector Buffer");
        // a.matrix_buffer.gpu_buffers[0]
        //     .buffer
        //     .set_name("Matrix Buffer");
        // a.indirect_buffer
        //     .gpu_indexed_indirect_buffer
        //     .buffer
        //     .set_name("Indexed Indirect Buffer");

        a
    }

    pub fn load_chunk(
        &mut self,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        command_buffer: vk::CommandBuffer,
        descriptors: &Descriptors,
        vector_descriptor_set: DescriptorSet,
    ) -> Option<ChunkGpuHandle> {
        let vert_bytes = bytemuck::cast_slice(&primitive.vertices);
        let idx_bytes = bytemuck::cast_slice(&primitive.indices);
        let vec_bytes = bytemuck::cast_slice(&vector);

        if vert_bytes.len() == 0 {
            return None;
        }

        let render_alloc = self.chunk_render_buffer_manager.alloc(
            device,
            mem_properties,
            vert_bytes.len() as u64,
            idx_bytes.len() as u64,
        );

        let vector_alloc = self.vector_buffer_manager.alloc(
            device,
            mem_properties,
            descriptors,
            vector_descriptor_set,
        );

        let vert_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vert_bytes);

        let idx_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(idx_bytes);

        let vec_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vec_bytes);

        self.chunk_render_buffer_manager.write_indirect(
            render_alloc.buffer_idx,
            render_alloc.single_alloc_data.indirect_alloc,
            DrawIndexedIndirectCommand {
                index_count: primitive.indices.len() as u32,
                instance_count: 1,
                first_index: (render_alloc
                    .single_alloc_data
                    .indices_offset
                    .get_bytes_offset()
                    / 4) as u32,
                vertex_offset: (render_alloc
                    .single_alloc_data
                    .vertices_offset
                    .get_bytes_offset()
                    / std::mem::size_of::<BlockVertex>()) as i32,
                first_instance: vector_alloc.get_idx() as u32,
            },
        );

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                render_alloc.single_alloc_data.vertices_buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vert_src_offset)
                    .dst_offset(
                        render_alloc
                            .single_alloc_data
                            .vertices_offset
                            .get_bytes_offset() as u64,
                    )
                    .size(vert_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                render_alloc.single_alloc_data.indices_buffer,
                &[vk::BufferCopy::default()
                    .src_offset(idx_src_offset)
                    .dst_offset(
                        render_alloc
                            .single_alloc_data
                            .indices_offset
                            .get_bytes_offset() as u64,
                    )
                    .size(idx_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                vector_alloc.single_alloc_data.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vec_src_offset)
                    .dst_offset(vector_alloc.single_alloc_data.slot_alloc.get_bytes_offset() as u64)
                    .size(vec_bytes.len() as u64)],
            );
        }

        Some(ChunkGpuHandle {
            render_alloc,
            vector_alloc,
        })
    }

    pub fn load_phys_object(
        &mut self,
        primitive: BlockIndexedPrimitive,
        mut vector: [i32; 4],
        matrix: [[f32; 4]; 4],
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        command_buffer: vk::CommandBuffer,
        descriptors: &Descriptors,
        vector_descriptor_set: DescriptorSet,
        matrix_descriptor_set: DescriptorSet,
    ) -> Option<PhysObjectGpuHandle> {
        let vert_bytes = bytemuck::cast_slice(&primitive.vertices);
        let idx_bytes = bytemuck::cast_slice(&primitive.indices);
        let mat_bytes = bytemuck::cast_slice(&matrix);

        if vert_bytes.len() == 0 {
            return None;
        }

        let render_alloc = self.phys_render_buffer_manager.alloc(
            device,
            mem_properties,
            vert_bytes.len() as u64,
            idx_bytes.len() as u64,
        );

        let vector_alloc = self.vector_buffer_manager.alloc(
            device,
            mem_properties,
            descriptors,
            vector_descriptor_set,
        );

        let matrix_alloc = self.matrix_buffer_manager.alloc(
            device,
            mem_properties,
            descriptors,
            matrix_descriptor_set,
        );

        vector[3] = matrix_alloc.get_idx() as i32;

        let vec_bytes = bytemuck::cast_slice(&vector);

        let vert_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vert_bytes);

        let idx_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(idx_bytes);

        let vec_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vec_bytes);

        let mat_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(mat_bytes);

        self.chunk_render_buffer_manager.write_indirect(
            render_alloc.buffer_idx,
            render_alloc.single_alloc_data.indirect_alloc,
            DrawIndexedIndirectCommand {
                index_count: primitive.indices.len() as u32,
                instance_count: 1,
                first_index: (render_alloc
                    .single_alloc_data
                    .indices_offset
                    .get_bytes_offset()
                    / 4) as u32,
                vertex_offset: (render_alloc
                    .single_alloc_data
                    .vertices_offset
                    .get_bytes_offset()
                    / std::mem::size_of::<BlockVertex>()) as i32,
                first_instance: vector_alloc.get_idx() as u32,
            },
        );

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                render_alloc.single_alloc_data.vertices_buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vert_src_offset)
                    .dst_offset(
                        render_alloc
                            .single_alloc_data
                            .vertices_offset
                            .get_bytes_offset() as u64,
                    )
                    .size(vert_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                render_alloc.single_alloc_data.indices_buffer,
                &[vk::BufferCopy::default()
                    .src_offset(idx_src_offset)
                    .dst_offset(
                        render_alloc
                            .single_alloc_data
                            .indices_offset
                            .get_bytes_offset() as u64,
                    )
                    .size(idx_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                vector_alloc.single_alloc_data.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vec_src_offset)
                    .dst_offset(vector_alloc.single_alloc_data.slot_alloc.get_bytes_offset() as u64)
                    .size(vec_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                matrix_alloc.single_alloc_data.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(mat_src_offset)
                    .dst_offset(matrix_alloc.single_alloc_data.slot_alloc.get_bytes_offset() as u64)
                    .size(mat_bytes.len() as u64)],
            );
        }

        Some(PhysObjectGpuHandle {
            render_alloc,
            vector_alloc,
            matrix_alloc,
        })
    }

    pub fn update_phys_object(
        &mut self,
        handle: PhysObjectGpuHandle,
        mut vector: [i32; 4],
        matrix: [[f32; 4]; 4],
        device: &ash::Device,
        command_buffer: CommandBuffer,
    ) {
        let vector_alloc = handle.vector_alloc;
        let matrix_alloc = handle.matrix_alloc;

        vector[3] = handle.matrix_alloc.get_idx() as i32;

        let mat_bytes = bytemuck::cast_slice(&matrix);
        let vec_bytes = bytemuck::cast_slice(&vector);

        let vec_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vec_bytes);

        let mat_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(mat_bytes);

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                vector_alloc.single_alloc_data.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vec_src_offset)
                    .dst_offset(vector_alloc.single_alloc_data.slot_alloc.get_bytes_offset() as u64)
                    .size(vec_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                matrix_alloc.single_alloc_data.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(mat_src_offset)
                    .dst_offset(matrix_alloc.single_alloc_data.slot_alloc.get_bytes_offset() as u64)
                    .size(mat_bytes.len() as u64)],
            );
        }
    }

    pub fn load_hand(
        &mut self,
        primitive: BlockIndexedPrimitive,
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        command_buffer: vk::CommandBuffer,
    ) -> Option<HandGpuHandle> {
        let vert_bytes = bytemuck::cast_slice(&primitive.vertices);
        let idx_bytes = bytemuck::cast_slice(&primitive.indices);

        if vert_bytes.len() == 0 {
            return None;
        }

        let render_alloc = self.hand_render_buffer_manager.alloc(
            device,
            mem_properties,
            vert_bytes.len() as u64,
            idx_bytes.len() as u64,
        );

        let vert_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vert_bytes);

        let idx_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(idx_bytes);

        self.chunk_render_buffer_manager.write_indirect(
            render_alloc.buffer_idx,
            render_alloc.single_alloc_data.indirect_alloc,
            DrawIndexedIndirectCommand {
                index_count: primitive.indices.len() as u32,
                instance_count: 1,
                first_index: (render_alloc
                    .single_alloc_data
                    .indices_offset
                    .get_bytes_offset()
                    / 4) as u32,
                vertex_offset: (render_alloc
                    .single_alloc_data
                    .vertices_offset
                    .get_bytes_offset()
                    / std::mem::size_of::<BlockVertex>()) as i32,
                first_instance: 0,
            },
        );

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                render_alloc.single_alloc_data.vertices_buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vert_src_offset)
                    .dst_offset(
                        render_alloc
                            .single_alloc_data
                            .vertices_offset
                            .get_bytes_offset() as u64,
                    )
                    .size(vert_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                render_alloc.single_alloc_data.indices_buffer,
                &[vk::BufferCopy::default()
                    .src_offset(idx_src_offset)
                    .dst_offset(
                        render_alloc
                            .single_alloc_data
                            .indices_offset
                            .get_bytes_offset() as u64,
                    )
                    .size(idx_bytes.len() as u64)],
            );
        }

        Some(HandGpuHandle { render_alloc })
    }

    pub fn unload_chunk(&mut self, data: ChunkGpuHandle) {
        self.chunk_render_buffer_manager.free(data.render_alloc);
        self.vector_buffer_manager.free(data.vector_alloc);
    }

    pub fn unload_phys_object(&mut self, data: PhysObjectGpuHandle) {
        self.phys_render_buffer_manager.free(data.render_alloc);
        self.vector_buffer_manager.free(data.vector_alloc);
        self.matrix_buffer_manager.free(data.matrix_alloc);
    }

    pub fn unload_hand(&mut self, data: HandGpuHandle) {
        self.hand_render_buffer_manager.free(data.render_alloc);
    }

    pub fn prepare_buffers(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        self.chunk_render_buffer_manager
            .write_staging(&mut self.cpu_staging_queue, &mut self.cpu_staging_buffer);
        self.phys_render_buffer_manager
            .write_staging(&mut self.cpu_staging_queue, &mut self.cpu_staging_buffer);
        self.hand_render_buffer_manager
            .write_staging(&mut self.cpu_staging_queue, &mut self.cpu_staging_buffer);

        self.cpu_staging_queue.drain(..).for_each(|cmd| unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                cmd.dst,
                &[vk::BufferCopy::default()
                    .src_offset(cmd.src_offset)
                    .dst_offset(cmd.dst_offset)
                    .size(cmd.length)],
            );
        });

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.cpu_staging_buffer.as_ptr() as *const std::ffi::c_void,
                self.gpu_staging_buffer.mapped_ptr,
                self.cpu_staging_buffer.len(),
            );
        }

        let mut barriers: Vec<_> = Vec::new();

        self.chunk_render_buffer_manager
            .barrier_queue
            .drain(..)
            .for_each(|barrier_data| {
                barriers.push(
                    vk::BufferMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(barrier_data.dst_access_mask) // Защищаем чтение indirect-команд
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .buffer(barrier_data.buffer)
                        .offset(0)
                        .size(vk::WHOLE_SIZE),
                );
            });

        self.phys_render_buffer_manager
            .barrier_queue
            .drain(..)
            .for_each(|barrier_data| {
                barriers.push(
                    vk::BufferMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(barrier_data.dst_access_mask) // Защищаем чтение indirect-команд
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .buffer(barrier_data.buffer)
                        .offset(0)
                        .size(vk::WHOLE_SIZE),
                );
            });

        self.hand_render_buffer_manager
            .barrier_queue
            .drain(..)
            .for_each(|barrier_data| {
                barriers.push(
                    vk::BufferMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(barrier_data.dst_access_mask) // Защищаем чтение indirect-команд
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .buffer(barrier_data.buffer)
                        .offset(0)
                        .size(vk::WHOLE_SIZE),
                );
            });

        self.vector_buffer_manager
            .barrier_queue
            .drain(..)
            .for_each(|barrier_data| {
                barriers.push(
                    vk::BufferMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(barrier_data.dst_access_mask) // Защищаем чтение indirect-команд
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .buffer(barrier_data.buffer)
                        .offset(0)
                        .size(vk::WHOLE_SIZE),
                );
            });

        self.matrix_buffer_manager
            .barrier_queue
            .drain(..)
            .for_each(|barrier_data| {
                barriers.push(
                    vk::BufferMemoryBarrier::default()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(barrier_data.dst_access_mask) // Защищаем чтение indirect-команд
                        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                        .buffer(barrier_data.buffer)
                        .offset(0)
                        .size(vk::WHOLE_SIZE),
                );
            });
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER, // Стадия-источник (копирование)
                vk::PipelineStageFlags::VERTEX_INPUT
                    | vk::PipelineStageFlags::DRAW_INDIRECT
                    | vk::PipelineStageFlags::VERTEX_SHADER, // Стадии-потребители (ввод геометрии и индирект)
                vk::DependencyFlags::empty(),
                &[],
                &barriers,
                &[],
            );
        }

        self.cpu_staging_buffer.clear();
    }
}

#[derive(Clone, Copy)]
pub struct ChunkGpuHandle {
    pub render_alloc: RenderAllocData,
    pub vector_alloc: SlotAllocData<{ size_of::<[i32; 4]>() }>,
}

#[derive(Clone, Copy)]
pub struct HandGpuHandle {
    pub render_alloc: RenderAllocData,
}

#[derive(Clone, Copy)]
pub struct PhysObjectGpuHandle {
    pub render_alloc: RenderAllocData,
    pub vector_alloc: SlotAllocData<{ size_of::<[i32; 4]>() }>,
    pub matrix_alloc: SlotAllocData<{ size_of::<[[f32; 4]; 4]>() }>,
}
