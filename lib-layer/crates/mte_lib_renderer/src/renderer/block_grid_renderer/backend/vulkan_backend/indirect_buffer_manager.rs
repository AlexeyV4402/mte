use ash::vk;
use ash::vk::*;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_manager::{
    PageAllocData, PageBuffer, SlotAllocData, SlotBuffer
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::{
    VkBufferDataDL, VkBufferDataHV
};
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use crate::renderer::block_grid_renderer::types::BlockVertex;

static mut COUNTER: i32 = 0;

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

    pub indirect_buffer: IndirectBuffer,

    pub vertex_buffer: PageBuffer<1024, 4096>,
    pub index_buffer: PageBuffer<1024, 4096>,
    pub matrix_buffer: SlotBuffer,
    pub vector_buffer: SlotBuffer,
}

impl IndirectBufferManager {
    pub const ONE_VERTEX_BUFFER_PAGE_CAPACITY: u64 = 4 * 1024;
    pub const ONE_VERTEX_BUFFER_PAGE_COUNT: u64 = 64 * 1024;
    pub const ONE_VERTEX_BUFFER_CAPACITY: u64 =
        Self::ONE_VERTEX_BUFFER_PAGE_CAPACITY * Self::ONE_VERTEX_BUFFER_PAGE_COUNT;

    pub const ONE_INDEX_BUFFER_PAGE_CAPACITY: u64 = 4 * 1024;
    pub const ONE_INDEX_BUFFER_PAGE_COUNT: u64 = 64 * 1024;
    pub const ONE_INDEX_BUFFER_CAPACITY: u64 =
        Self::ONE_INDEX_BUFFER_PAGE_CAPACITY * Self::ONE_INDEX_BUFFER_PAGE_COUNT;

    pub const ONE_MATRIX_BUFFER_SLOTS_COUNT: u64 = 1;
    pub const ONE_MATRIX_BUFFER_CAPACITY: u64 =
        Self::ONE_MATRIX_BUFFER_SLOTS_COUNT * (size_of::<[[f32; 4]; 4]>() as u64);

    pub const ONE_VECTOR_BUFFER_SLOTS_COUNT: u64 = 256;
    pub const ONE_VECTOR_BUFFER_CAPACITY: u64 =
        Self::ONE_VECTOR_BUFFER_SLOTS_COUNT * (size_of::<[i32; 4]>() as u64);

    pub const STAGING_BUFFER_CAPACITY: u64 = 1024 * 1024 * 1024;

    pub const INDIRECT_BUFFER_SLOTS_COUNT: u64 =
        Self::ONE_VECTOR_BUFFER_SLOTS_COUNT + Self::ONE_MATRIX_BUFFER_SLOTS_COUNT;
    pub const INDIRECT_BUFFER_CAPACITY: u64 =
        Self::INDIRECT_BUFFER_SLOTS_COUNT * (size_of::<DrawIndexedIndirectCommand>() as u64);

    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        Self {
            gpu_staging_buffer: VkBufferDataHV::new(
                device,
                mem_properties,
                Self::STAGING_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_SRC,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            ),
            cpu_staging_buffer: Vec::with_capacity(Self::STAGING_BUFFER_CAPACITY as usize),
            cpu_staging_queue: Vec::new(),
            indirect_buffer: IndirectBuffer {
                gpu_indexed_indirect_buffer: VkBufferDataDL::new(
                    device,
                    mem_properties,
                    Self::INDIRECT_BUFFER_CAPACITY,
                    BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::INDIRECT_BUFFER,
                    MemoryPropertyFlags::DEVICE_LOCAL,
                ),
                cpu_indexed_indirect_buffer: vec![
                    DrawIndexedIndirectCommand::default();
                    Self::INDIRECT_BUFFER_SLOTS_COUNT as usize
                ],
                indexed_indirect_buffer_free_slots: (1..Self::INDIRECT_BUFFER_SLOTS_COUNT as usize - 1)
                    .collect(),
            },

            vertex_buffer: PageBuffer::new(
                device,
                mem_properties,
                Self::ONE_VERTEX_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::VERTEX_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            index_buffer: PageBuffer::new(
                device,
                mem_properties,
                Self::ONE_INDEX_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::INDEX_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            matrix_buffer: SlotBuffer::new(
                device,
                mem_properties,
                Self::ONE_MATRIX_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            vector_buffer: SlotBuffer::new(
                device,
                mem_properties,
                Self::ONE_VECTOR_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
        }
    }

    pub fn load_chunk(
        &mut self,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        command_buffer: vk::CommandBuffer,
    ) -> Option<ChunkGpuHandle> {
        let vert_bytes = bytemuck::cast_slice(&primitive.vertices);
        let idx_bytes = bytemuck::cast_slice(&primitive.indices);
        let vec_bytes = bytemuck::cast_slice(&vector);

        if vert_bytes.len() == 0 {
            return None;
        }

        let vertex_alloc =
            self.vertex_buffer
                .alloc(vert_bytes.len() as u64, device, mem_properties);

        let index_alloc = self
            .index_buffer
            .alloc(idx_bytes.len() as u64, device, mem_properties);

        let vector_alloc = self.vector_buffer.alloc(device, mem_properties);

        let indirect_alloc = self.indirect_buffer.alloc();

        let vert_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vert_bytes);

        let idx_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(idx_bytes);

        let vec_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vec_bytes);

        self.indirect_buffer.write(
            indirect_alloc,
            DrawIndexedIndirectCommand {
                index_count: primitive.indices.len() as u32,
                instance_count: 1,
                first_index: (index_alloc.offset / 4) as u32,
                vertex_offset: (vertex_alloc.offset / std::mem::size_of::<BlockVertex>() as u64)
                    as i32,
                first_instance: vector_alloc.slot_idx,
            },
        );

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                vertex_alloc.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vert_src_offset)
                    .dst_offset(vertex_alloc.offset)
                    .size(vert_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                index_alloc.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(idx_src_offset)
                    .dst_offset(index_alloc.offset)
                    .size(idx_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                vector_alloc.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vec_src_offset)
                    .dst_offset(vector_alloc.offset)
                    .size(vec_bytes.len() as u64)],
            );
        }

        Some(ChunkGpuHandle {
            vertex_alloc,
            index_alloc,
            vector_alloc,
            indirect_alloc,
        })
    }

    pub fn load_hand(
        &mut self,
        primitive: BlockIndexedPrimitive,
        matrix: [[f32; 4]; 4],
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        command_buffer: vk::CommandBuffer,
    ) -> Option<HandGpuHandle> {
        let vert_bytes = bytemuck::cast_slice(&primitive.vertices);
        let idx_bytes = bytemuck::cast_slice(&primitive.indices);
        let mat_bytes = bytemuck::cast_slice(&matrix);

        if vert_bytes.len() == 0 {
            return None;
        }

        let vertex_alloc =
            self.vertex_buffer
                .alloc(vert_bytes.len() as u64, device, mem_properties);

        let index_alloc = self
            .index_buffer
            .alloc(idx_bytes.len() as u64, device, mem_properties);

        let matrix_alloc = self.matrix_buffer.alloc(device, mem_properties);

        let mut indirect_alloc = self.indirect_buffer.alloc();
        indirect_alloc.idx = 0;

        let vert_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(vert_bytes);

        let idx_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(idx_bytes);

        let mat_src_offset = self.cpu_staging_buffer.len() as u64;
        self.cpu_staging_buffer.extend_from_slice(mat_bytes);

        self.indirect_buffer.write(
            indirect_alloc,
            DrawIndexedIndirectCommand {
                index_count: primitive.indices.len() as u32,
                instance_count: 1,
                first_index: (index_alloc.offset / 4) as u32,
                vertex_offset: (vertex_alloc.offset / std::mem::size_of::<BlockVertex>() as u64)
                    as i32,
                first_instance: matrix_alloc.slot_idx,
            },
        );

        unsafe {
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                vertex_alloc.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(vert_src_offset)
                    .dst_offset(vertex_alloc.offset)
                    .size(vert_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                index_alloc.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(idx_src_offset)
                    .dst_offset(index_alloc.offset)
                    .size(idx_bytes.len() as u64)],
            );
            device.cmd_copy_buffer(
                command_buffer,
                self.gpu_staging_buffer.buffer,
                matrix_alloc.buffer,
                &[vk::BufferCopy::default()
                    .src_offset(mat_src_offset)
                    .dst_offset(matrix_alloc.offset)
                    .size(mat_bytes.len() as u64)],
            );
        }

        Some(HandGpuHandle {
            vertex_alloc,
            index_alloc,
            matrix_alloc,
            indirect_alloc,
        })
    }

    pub fn prepare_buffers(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        let command_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.indirect_buffer.cpu_indexed_indirect_buffer.as_ptr() as *const u8,
                self.indirect_buffer.cpu_indexed_indirect_buffer.len()
                    * std::mem::size_of::<vk::DrawIndexedIndirectCommand>(),
            )
        };

        self.cpu_staging_queue.push(StagingBufferCommand {
            src_offset: self.cpu_staging_buffer.len() as u64,
            dst_offset: 0,
            length: (self.indirect_buffer.cpu_indexed_indirect_buffer.len()
                * std::mem::size_of::<vk::DrawIndexedIndirectCommand>()) as u64,
            dst: self.indirect_buffer.gpu_indexed_indirect_buffer.buffer,
        });

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

        self.cpu_staging_buffer.clear();
    }

    pub fn unload_chunk(&mut self, data: ChunkGpuHandle) {
        self.vertex_buffer.free(data.vertex_alloc);
        self.index_buffer.free(data.index_alloc);
        self.vector_buffer.free(data.vector_alloc);
        self.indirect_buffer.free(data.indirect_alloc);
    }

    pub fn unload_hand(&mut self, data: HandGpuHandle) {
        self.vertex_buffer.free(data.vertex_alloc);
        self.index_buffer.free(data.index_alloc);
        self.matrix_buffer.free(data.matrix_alloc);
        self.indirect_buffer.free(data.indirect_alloc);
    }
}

#[derive(Clone, Copy)]
pub struct ChunkGpuHandle {
    pub vertex_alloc: PageAllocData,
    pub index_alloc: PageAllocData,
    pub vector_alloc: SlotAllocData,
    pub indirect_alloc: IndirectAllocData,
}

#[derive(Clone, Copy)]
pub struct HandGpuHandle {
    pub vertex_alloc: PageAllocData,
    pub index_alloc: PageAllocData,
    pub matrix_alloc: SlotAllocData,
    pub indirect_alloc: IndirectAllocData,
}

pub struct IndirectBuffer {
    pub gpu_indexed_indirect_buffer: VkBufferDataDL,
    pub cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectCommand>,

    pub indexed_indirect_buffer_free_slots: Vec<usize>,
}

#[derive(Clone, Copy)]
pub struct IndirectAllocData {
    buffer: vk::Buffer,
    idx: usize,
}

impl IndirectBuffer {
    pub fn write(
        &mut self,
        alloc_data: IndirectAllocData,
        indirect_cmd: DrawIndexedIndirectCommand,
    ) {
        self.cpu_indexed_indirect_buffer[alloc_data.idx] = indirect_cmd;
    }

    pub fn alloc(&mut self) -> IndirectAllocData {
        return IndirectAllocData {
            buffer: self.gpu_indexed_indirect_buffer.buffer,
            idx: self.indexed_indirect_buffer_free_slots.pop().unwrap(),
        };
    }

    pub fn free(&mut self, data: IndirectAllocData) {
        self.cpu_indexed_indirect_buffer[data.idx] = DrawIndexedIndirectCommand {
            index_count: 0,
            instance_count: 0,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        };
        self.indexed_indirect_buffer_free_slots.push(data.idx);
    }
}
