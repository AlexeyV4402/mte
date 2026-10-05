use ash::vk;
use ash::vk::*;
use lib_core::alloc_helper::{ConstPageAllocHelper, SectorAlloc};

use crate::renderer::block_grid_renderer::backend::vulkan_backend::indirect_buffer_manager::{
    IndirectBufferManager, StagingBufferCommand
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataDL;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::descriptors::Descriptors;

#[derive(Clone, Copy)]
pub struct RenderAllocData {
    pub buffer_idx: usize,
    pub single_alloc_data: SingleRenderAllocData,
}

#[derive(PartialEq)]
pub struct BarrierData {
    pub dst_access_mask: vk::AccessFlags,
    pub buffer: vk::Buffer,
}

pub struct RenderBufferManager {
    pub buffers: Vec<RenderBuffer>,
    pub barrier_queue: Vec<BarrierData>,
    indirect_update_queue: Vec<usize>,
}

impl RenderBufferManager {
    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        let mut buffers = Vec::with_capacity(2);
        buffers.push(RenderBuffer::new(device, mem_properties));
        Self {
            buffers,
            barrier_queue: Vec::new(),
            indirect_update_queue: Vec::new(),
        }
    }

    pub fn write_indirect(
        &mut self,
        buffer_idx: usize,
        alloc_idx: usize,
        cmd: DrawIndexedIndirectCommand,
    ) {
        self.buffers[buffer_idx].cpu_indirect_buffer[alloc_idx] = cmd;
    }

    pub fn clear_indirect(&mut self, buffer_idx: usize, alloc_idx: usize) {
        self.buffers[buffer_idx].cpu_indirect_buffer[alloc_idx] =
            DrawIndexedIndirectCommand::default();
    }

    pub fn alloc(
        &mut self,
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        vertices_bytes: u64,
        indices_bytes: u64,
    ) -> RenderAllocData {
        let mut alloc_data: Option<SingleRenderAllocData> = None;
        let mut idx = 0;
        for (i, buffer) in &mut self.buffers.iter_mut().enumerate() {
            alloc_data = buffer.alloc(vertices_bytes, indices_bytes);
            if alloc_data.is_some() {
                idx = i;
                break;
            }
        }
        if alloc_data.is_none() {
            idx = self.buffers.len();
            self.buffers.push(RenderBuffer::new(device, mem_properties));
            alloc_data = self.buffers[idx].alloc(vertices_bytes, indices_bytes);
        };
        let alloc_data = alloc_data.unwrap();
        if !self.barrier_queue.contains(&BarrierData {
            dst_access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            buffer: alloc_data.vertices_buffer,
        }) {
            self.barrier_queue.push(BarrierData {
                dst_access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
                buffer: alloc_data.vertices_buffer,
            });
            self.barrier_queue.push(BarrierData {
                dst_access_mask: vk::AccessFlags::INDEX_READ,
                buffer: alloc_data.indices_buffer,
            });
            self.barrier_queue.push(BarrierData {
                dst_access_mask: vk::AccessFlags::INDIRECT_COMMAND_READ,
                buffer: alloc_data.indirect_buffer,
            });
            self.indirect_update_queue.push(idx);
        }
        RenderAllocData {
            buffer_idx: idx,
            single_alloc_data: alloc_data,
        }
    }

    pub fn free(&mut self, alloc_data: RenderAllocData) {
        self.buffers[alloc_data.buffer_idx].free(alloc_data.single_alloc_data);
        if !self.barrier_queue.contains(&BarrierData {
            dst_access_mask: vk::AccessFlags::INDIRECT_COMMAND_READ,
            buffer: alloc_data.single_alloc_data.indirect_buffer,
        }) {
            self.barrier_queue.push(BarrierData {
                dst_access_mask: vk::AccessFlags::INDIRECT_COMMAND_READ,
                buffer: alloc_data.single_alloc_data.indirect_buffer,
            });
            self.indirect_update_queue.push(alloc_data.buffer_idx);
        }
    }

    pub fn destroy(self, device: &ash::Device) {
        self.buffers.into_iter().for_each(|buffer| {
            buffer.destroy(device);
        });
    }

    pub fn write_staging(
        &mut self,
        cpu_staging_queue: &mut Vec<StagingBufferCommand>,
        cpu_staging_buffer: &mut Vec<u8>,
    ) {
        self.indirect_update_queue.drain(..).for_each(|buffer_idx| {
            let gpu_indirect_buffer = self.buffers[buffer_idx].indirect_buffer.buffer;
            let cpu_indirect_buffer = &self.buffers[buffer_idx].cpu_indirect_buffer;
            cpu_staging_queue.push(StagingBufferCommand {
                src_offset: cpu_staging_buffer.len() as u64,
                dst_offset: 0,
                length: (self.buffers[buffer_idx].cpu_indirect_buffer.len()
                    * std::mem::size_of::<vk::DrawIndexedIndirectCommand>())
                    as u64,
                dst: gpu_indirect_buffer,
            });

            let command_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    cpu_indirect_buffer.as_ptr() as *const u8,
                    cpu_indirect_buffer.len()
                        * std::mem::size_of::<vk::DrawIndexedIndirectCommand>(),
                )
            };

            cpu_staging_buffer.extend_from_slice(command_bytes);
        });
    }
}

pub struct RenderBuffer {
    pub vertex_buffer: VkBufferDataDL,
    pub index_buffer: VkBufferDataDL,
    pub indirect_buffer: VkBufferDataDL,
    pub vertex_alloc_helper: ConstPageAllocHelper<1024>,
    pub index_alloc_helper: ConstPageAllocHelper<1024>,
    pub inderect_alloc_helper: Vec<usize>,
    pub cpu_indirect_buffer: Vec<DrawIndexedIndirectCommand>,
}

#[derive(Clone, Copy)]
pub struct SingleRenderAllocData {
    pub vertices_buffer: vk::Buffer,
    pub vertices_offset: SectorAlloc<{ RenderBuffer::VERTICES_SECTOR_SIZE as usize }>,
    pub vertices_sector_count: usize,
    pub indices_buffer: vk::Buffer,
    pub indices_offset: SectorAlloc<{ RenderBuffer::INDICES_SECTOR_SIZE as usize }>,
    pub indices_sector_count: usize,
    pub indirect_buffer: vk::Buffer,
    pub indirect_alloc: usize,
}

impl RenderBuffer {
    pub const VERTICES_SECTOR_SIZE: u64 = IndirectBufferManager::ONE_VERTEX_BUFFER_PAGE_CAPACITY;
    pub const INDICES_SECTOR_SIZE: u64 = IndirectBufferManager::ONE_INDEX_BUFFER_PAGE_CAPACITY;

    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        Self {
            vertex_buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                IndirectBufferManager::ONE_VERTEX_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::VERTEX_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            index_buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                IndirectBufferManager::ONE_INDEX_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::INDEX_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            indirect_buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                IndirectBufferManager::ONE_INDIRECT_BUFFER_CAPACITY,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::INDIRECT_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            vertex_alloc_helper: ConstPageAllocHelper::default(),
            index_alloc_helper: ConstPageAllocHelper::default(),
            inderect_alloc_helper: (0..IndirectBufferManager::ONE_INDIRECT_BUFFER_SLOTS_COUNT
                as usize)
                .collect(),
            cpu_indirect_buffer: vec![
                DrawIndexedIndirectCommand::default();
                IndirectBufferManager::ONE_INDIRECT_BUFFER_SLOTS_COUNT
                    as usize
            ],
        }
    }

    pub fn alloc(
        &mut self,
        vertices_bytes: u64,
        indices_bytes: u64,
    ) -> Option<SingleRenderAllocData> {
        let vertices_sector_count = vertices_bytes.div_ceil(Self::VERTICES_SECTOR_SIZE) as usize;
        let vertices_alloc = self
            .vertex_alloc_helper
            .alloc::<{ Self::VERTICES_SECTOR_SIZE as usize }>(vertices_sector_count);
        if vertices_alloc.get_sectors_offset() > self.vertex_alloc_helper.get_page_count() + 1 {
            return None;
        };
        let indices_sector_count = indices_bytes.div_ceil(Self::INDICES_SECTOR_SIZE) as usize;
        let indices_alloc = self
            .index_alloc_helper
            .alloc::<{ Self::INDICES_SECTOR_SIZE as usize }>(indices_sector_count);
        if indices_alloc.get_sectors_offset() > self.index_alloc_helper.get_page_count() + 1 {
            return None;
        };
        let indirect_alloc = match self.inderect_alloc_helper.pop() {
            Some(n) => n,
            None => return None,
        };
        Some(SingleRenderAllocData {
            vertices_buffer: self.vertex_buffer.buffer,
            vertices_offset: vertices_alloc,
            vertices_sector_count,
            indices_buffer: self.index_buffer.buffer,
            indices_offset: indices_alloc,
            indices_sector_count,
            indirect_buffer: self.indirect_buffer.buffer,
            indirect_alloc,
        })
    }

    pub fn free(&mut self, alloc_data: SingleRenderAllocData) {
        self.vertex_alloc_helper.free(
            alloc_data.vertices_offset.get_sectors_offset(),
            alloc_data.vertices_sector_count,
        );
        self.index_alloc_helper.free(
            alloc_data.indices_offset.get_sectors_offset(),
            alloc_data.indices_sector_count,
        );
        self.inderect_alloc_helper.push(alloc_data.indirect_alloc);
    }

    pub fn destroy(self, device: &ash::Device) {
        self.vertex_buffer.destroy(device);
        self.index_buffer.destroy(device);
        self.indirect_buffer.destroy(device);
        drop(self.vertex_alloc_helper);
        drop(self.index_alloc_helper);
        drop(self.inderect_alloc_helper);
    }

    pub fn load_indirect(&self) {}
}

pub struct SlotBufferManager<const SLOT_SIZE: usize, const SLOT_COUNT: usize> {
    pub buffers: Vec<SlotBuffer<SLOT_SIZE, SLOT_COUNT>>,
    pub barrier_queue: Vec<BarrierData>,
}

#[derive(Clone, Copy)]
pub struct SlotAllocData<const SLOT_SIZE: usize> {
    buffer_idx: usize,
    pub single_alloc_data: SingleSlotAllocData<SLOT_SIZE>,
}

impl<const SLOT_SIZE: usize> SlotAllocData<SLOT_SIZE> {
    #[inline]
    pub fn get_idx(&self) -> usize {
        self.buffer_idx << 16 | self.single_alloc_data.slot_alloc.get_slots_offset()
    }
}

impl<const SLOT_SIZE: usize, const SLOT_COUNT: usize> SlotBufferManager<SLOT_SIZE, SLOT_COUNT> {
    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        let mut buffers = Vec::with_capacity(2);
        buffers.push(SlotBuffer::new(device, mem_properties));
        Self {
            buffers,
            barrier_queue: Vec::new(),
        }
    }

    pub fn alloc(
        &mut self,
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        descriptors: &Descriptors,
        descriptor_set: DescriptorSet,
    ) -> SlotAllocData<SLOT_SIZE> {
        let mut alloc_data: Option<SingleSlotAllocData<SLOT_SIZE>> = None;
        let mut idx = 0;
        for (i, buffer) in &mut self.buffers.iter_mut().enumerate() {
            alloc_data = buffer.alloc();
            if alloc_data.is_some() {
                idx = i;
                break;
            }
        }
        if alloc_data.is_none() {
            idx = self.buffers.len();
            self.buffers.push(SlotBuffer::new(device, mem_properties));
            descriptors.update_pool(
                device,
                self.buffers[idx].buffer.buffer,
                descriptor_set,
                idx as u32,
            );
            alloc_data = self.buffers[idx].alloc();
        };
        let alloc_data = alloc_data.unwrap();
        if !self.barrier_queue.contains(&BarrierData {
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            buffer: alloc_data.buffer,
        }) {
            self.barrier_queue.push(BarrierData {
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                buffer: alloc_data.buffer,
            });
        }
        SlotAllocData {
            buffer_idx: idx,
            single_alloc_data: alloc_data,
        }
    }

    pub fn free(&mut self, alloc_data: SlotAllocData<SLOT_SIZE>) {
        if !self.barrier_queue.contains(&BarrierData {
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            buffer: alloc_data.single_alloc_data.buffer,
        }) {
            self.barrier_queue.push(BarrierData {
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                buffer: alloc_data.single_alloc_data.buffer,
            });
        }
    }

    pub fn destroy(self, device: &ash::Device) {
        self.buffers.into_iter().for_each(|buffer| {
            buffer.destroy(device);
        });
    }
}

#[derive(Clone, Copy)]
pub struct SingleSlotAllocData<const SLOT_SIZE: usize> {
    pub buffer: vk::Buffer,
    pub slot_alloc: SlotAlloc<SLOT_SIZE>,
}

#[derive(Clone, Copy)]
pub struct SlotAlloc<const SLOT_SIZE: usize>(usize);

impl<const SLOT_SIZE: usize> SlotAlloc<SLOT_SIZE> {
    pub fn get_bytes_offset(self) -> usize {
        self.0 * SLOT_SIZE
    }

    pub fn get_slots_offset(self) -> usize {
        self.0
    }
}

pub struct SlotBuffer<const SLOT_SIZE: usize, const SLOT_COUNT: usize> {
    pub buffer: VkBufferDataDL,
    alloc_helper: Vec<usize>,
}

impl<const SLOT_SIZE: usize, const SLOT_COUNT: usize> SlotBuffer<SLOT_SIZE, SLOT_COUNT> {
    pub fn new(device: &ash::Device, mem_properties: &vk::PhysicalDeviceMemoryProperties) -> Self {
        Self {
            buffer: VkBufferDataDL::new(
                device,
                mem_properties,
                (SLOT_SIZE * SLOT_COUNT) as u64,
                BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            alloc_helper: (0..IndirectBufferManager::ONE_INDIRECT_BUFFER_SLOTS_COUNT as usize)
                .collect(),
        }
    }

    pub fn alloc(&mut self) -> Option<SingleSlotAllocData<SLOT_SIZE>> {
        match self.alloc_helper.pop() {
            Some(n) => Some(SingleSlotAllocData {
                buffer: self.buffer.buffer,
                slot_alloc: SlotAlloc(n),
            }),
            None => None,
        }
    }

    pub fn free(&mut self, alloc_data: SingleSlotAllocData<SLOT_SIZE>) {
        self.alloc_helper
            .push(alloc_data.slot_alloc.get_slots_offset());
    }

    pub fn destroy(self, device: &ash::Device) {
        self.buffer.destroy(device);
        drop(self.alloc_helper);
    }
}
