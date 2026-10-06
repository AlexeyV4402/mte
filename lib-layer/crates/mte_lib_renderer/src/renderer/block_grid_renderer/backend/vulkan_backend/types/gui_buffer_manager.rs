use ash::vk;
use ash::vk::*;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_manager::render_buffer_manager::{RenderAllocData, RenderBufferManager};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_manager::slot_buffer_manager::{SlotAllocData, SlotBufferManager};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::{
    VkBufferDataDL, VkBufferDataHV
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::descriptors::Descriptors;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::game_buffer_manager::StagingBufferCommand;
use crate::renderer::block_grid_renderer::render_objects::primitive::{BlockIndexedPrimitive, GuiIndexedPrimitive};
use crate::renderer::block_grid_renderer::types::BlockVertex;
use crate::renderer::block_grid_renderer::types::vertex::GuiVertex;

pub struct GuiBufferManager {
    pub gpu_staging_buffer: VkBufferDataHV,
    pub cpu_staging_buffer: Vec<u8>,

    pub cpu_staging_queue: Vec<StagingBufferCommand>,

    pub vertex_buffer: VkBufferDataDL,
    pub index_buffer: VkBufferDataDL,
    pub gpu_indirect_buffer: VkBufferDataDL,

    pub cpu_indirect_buffer: Vec<DrawIndexedIndirectCommand>,
}

impl GuiBufferManager {
    pub const QUAD_COUNT: u64 = 100;

    pub const ONE_VERTEX_BUFFER_CAPACITY: u64 =
        Self::QUAD_COUNT * 4 * (size_of::<GuiVertex>() as u64);

    pub const ONE_INDEX_BUFFER_CAPACITY: u64 = Self::QUAD_COUNT * 6 * (size_of::<u32>() as u64);

    pub const STAGING_BUFFER_CAPACITY: u64 = 1024 * 1024;

    pub const ONE_INDIRECT_BUFFER_CAPACITY: u64 =
        Self::QUAD_COUNT * (size_of::<DrawIndexedIndirectCommand>() as u64);

    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        let obj = Self {
            gpu_staging_buffer: VkBufferDataHV::new(
                device,
                mem_properties,
                Self::STAGING_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_SRC,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            ),
            cpu_staging_buffer: Vec::with_capacity(Self::STAGING_BUFFER_CAPACITY as usize),
            cpu_staging_queue: Vec::new(),
            vertex_buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                Self::ONE_VERTEX_BUFFER_CAPACITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            index_buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                Self::ONE_INDEX_BUFFER_CAPACITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            gpu_indirect_buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                Self::ONE_VERTEX_BUFFER_CAPACITY,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDIRECT_BUFFER,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            cpu_indirect_buffer: vec![
                DrawIndexedIndirectCommand::default();
                Self::QUAD_COUNT as usize
            ],
        };
        obj
    }

    pub fn load_quad(
        &mut self,
        primitive: GuiIndexedPrimitive,
        idx: u64,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
    ) {
        let vert_bytes = bytemuck::cast_slice(&primitive.vertices);
        let idx_bytes = bytemuck::cast_slice(&primitive.indices);

        let vert_dst_offset = idx * 4 * (size_of::<GuiVertex>() as u64);
        let idx_dst_offset = idx * 6 * (size_of::<u32>() as u64);

        let vert_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vert_bytes);

        let idx_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(idx_bytes);

        self.cpu_indirect_buffer[idx as usize] = DrawIndexedIndirectCommand {
            index_count: 6,
            instance_count: 1,
            first_index: (idx * 6) as u32,
            vertex_offset: (idx * 4) as i32,
            first_instance: 0,
        };

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                self.vertex_buffer.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vert_src_offset)
                    .dst_offset(vert_dst_offset)
                    .size(vert_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                self.index_buffer.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(idx_src_offset)
                    .dst_offset(idx_dst_offset)
                    .size(idx_bytes.len() as u64)],
            );
        }
    }

    pub fn prepare_buffers(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        let gpu_indirect_buffer = self.gpu_indirect_buffer.buffer;
        let cpu_indirect_buffer = &self.cpu_indirect_buffer;
        self.cpu_staging_queue.push(StagingBufferCommand {
            src_offset: self.cpu_staging_buffer.len() as u64,
            dst_offset: 0,
            length: (self.cpu_indirect_buffer.len()
                * std::mem::size_of::<vk::DrawIndexedIndirectCommand>()) as u64,
            dst: gpu_indirect_buffer,
        });

        let command_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                cpu_indirect_buffer.as_ptr() as *const u8,
                cpu_indirect_buffer.len() * std::mem::size_of::<vk::DrawIndexedIndirectCommand>(),
            )
        };

        self.cpu_staging_buffer.extend_from_slice(command_bytes);

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

        barriers.push(
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ) // Защищаем чтение indirect-команд
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.vertex_buffer.buffer)
                .offset(0)
                .size(vk::WHOLE_SIZE),
        );

        barriers.push(
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::INDEX_READ) // Защищаем чтение indirect-команд
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.index_buffer.buffer)
                .offset(0)
                .size(vk::WHOLE_SIZE),
        );

        barriers.push(
            vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::INDIRECT_COMMAND_READ) // Защищаем чтение indirect-команд
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.gpu_indirect_buffer.buffer)
                .offset(0)
                .size(vk::WHOLE_SIZE),
        );

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

    pub fn destroy(self, device: &ash::Device) {
        self.gpu_indirect_buffer.destroy(device);
        self.gpu_staging_buffer.destroy(device);
        self.index_buffer.destroy(device);
        self.vertex_buffer.destroy(device);
        drop(self.cpu_staging_queue);
        drop(self.cpu_staging_buffer);
        drop(self.cpu_indirect_buffer);
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
