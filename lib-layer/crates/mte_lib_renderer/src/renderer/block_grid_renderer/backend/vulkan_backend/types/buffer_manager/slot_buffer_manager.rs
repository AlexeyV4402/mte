use ash::vk;
use ash::vk::*;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_manager::BarrierData;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataDL;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::descriptors::Descriptors;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::game_buffer_manager::GameBufferManager;

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
        // free внутри одиночного буфер
        self.buffers[alloc_data.buffer_idx].free(alloc_data.single_alloc_data);

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
            alloc_helper: (0..GameBufferManager::ONE_INDIRECT_BUFFER_SLOTS_COUNT as usize)
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
