use ash::vk::*;
use ash::{Entry, vk};
use lib_core::alloc_helper::ConstPageAllocHelper;
use mte_macros::vfs_include_vk_shader;
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use winit::dpi::PhysicalSize;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataDL;
use crate::renderer::block_grid_renderer::consts::{
    GLOBAL_INDEX_BUFFER_CAPACITY, GLOBAL_VERTEX_BUFFER_CAPACITY
};
use crate::renderer::block_grid_renderer::types::BlockVertex;

#[derive(Clone, Copy, Debug)]
pub struct PageAllocData {
    pub buffer: vk::Buffer,
    pub offset: vk::DeviceSize,

    pub buffer_idx: usize,
    pub sector_offset: usize,
    pub sector_count: usize,
}

pub struct PageBuffer<const ONE_BUFFER_BLOCKS: usize, const SECTOR_SIZE: u64> {
    pub gpu_buffers: Vec<VkBufferDataDL>,
    alloc_helper: Vec<ConstPageAllocHelper<ONE_BUFFER_BLOCKS>>,
}

impl<const ONE_BUFFER_BLOCKS: usize, const SECTOR_SIZE: u64>
    PageBuffer<ONE_BUFFER_BLOCKS, SECTOR_SIZE>
{
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
        Self {
            gpu_buffers: vec![VkBufferDataDL::new(
                device,
                mem_properties,
                size,
                usage,
                properties,
            )],
            alloc_helper: vec![ConstPageAllocHelper {
                bitset: [0; ONE_BUFFER_BLOCKS],
            }],
        }
    }

    pub fn alloc(
        &mut self,
        data_len: u64,
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> PageAllocData {
        let sectors_needed = data_len.div_ceil(SECTOR_SIZE) as usize;

        let max_sectors = ONE_BUFFER_BLOCKS * 64;

        // 2. Ищем место в уже существующих аллокаторах
        for (idx, helper) in self.alloc_helper.iter_mut().enumerate() {
            let sector_offset = helper.alloc(sectors_needed);
            // println!(
            //     "sector_offset: {}, sectors_needed: {}",
            //     sector_offset, sectors_needed
            // );

            // Если аллокатор нашел место (вернул индекс меньше макса)
            if sector_offset < max_sectors {
                return PageAllocData {
                    buffer: self.gpu_buffers[idx].buffer, // Берем буфер по тому же индексу
                    offset: (sector_offset as u64 * SECTOR_SIZE) as vk::DeviceSize,
                    buffer_idx: idx,
                    sector_offset,
                    sector_count: sectors_needed,
                };
            }
        }

        panic!("Кончились слоты");

        // 3. Если места не нашлось нигде — создаем НОВЫЙ мега-буфер
        let total_buffer_size = (max_sectors as u64 * SECTOR_SIZE) as vk::DeviceSize;

        // Вызываем создание буфера. Флаги зашиты намертво, чтобы не спамить в сигнатуру функции
        let new_gpu_buffer = VkBufferDataDL::new(
            device,
            mem_properties,
            total_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        // Создаем под него чистый битмап-аллокатор
        let mut new_helper = ConstPageAllocHelper::<ONE_BUFFER_BLOCKS> {
            bitset: [0; ONE_BUFFER_BLOCKS],
        };

        // Сразу забираем из него наши сектора (в новом буфере они 100% свободны)
        let sector_offset = new_helper.alloc(sectors_needed);

        // Синхронно пушим в оба вектора
        self.gpu_buffers.push(new_gpu_buffer);
        self.alloc_helper.push(new_helper);

        let new_idx = self.gpu_buffers.len() - 1;

        PageAllocData {
            buffer: self.gpu_buffers[new_idx].buffer,
            offset: (sector_offset as u64 * SECTOR_SIZE) as vk::DeviceSize,
            buffer_idx: new_idx,
            sector_offset,
            sector_count: sectors_needed,
        }
    }

    /// Освобождение памяти чанка при его выгрузке
    pub fn free(&mut self, alloc: PageAllocData) {
        if let Some(helper) = self.alloc_helper.get_mut(alloc.buffer_idx) {
            // Сбрасываем биты в битмапе обратно в 0
            helper.free(alloc.sector_offset, alloc.sector_count);
        } else {
            panic!("Неверный индекс буфера")
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SlotAllocData {
    pub buffer: vk::Buffer,
    pub offset: vk::DeviceSize,
    pub slot_idx: u32, // Индекс элемента внутри буфера (передается в шейдер)

    // Метаданные для деаллокации
    pub buffer_idx: usize,
    pub global_slot_id: u64,
}

pub struct SlotBuffer {
    pub gpu_buffers: Vec<VkBufferDataDL>,
    free_slots: Vec<u64>,      // Хранит глобальные ID свободных слотов
    slot_size: vk::DeviceSize, // Размер одного элемента с учетом выравнивания Vulkan
    // Копируем параметры создания, чтобы рожать новые буферы при нехватке места
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
}

impl SlotBuffer {
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        total_size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
        Self {
            gpu_buffers: vec![VkBufferDataDL::new(
                device,
                mem_properties,
                total_size,
                usage,
                properties,
            )],
            free_slots: (0..256).collect(),
            slot_size: 16,
            usage,
            properties,
        }
    }

    pub fn alloc(
        &mut self,
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> SlotAllocData {
        // 1. Если свободные слоты кончились, создаем еще один буфер на 64 места
        if self.free_slots.is_empty() {
            panic!("Кончились слоты");
            let total_size = self.slot_size * 256;
            let new_buffer = VkBufferDataDL::new(
                device,
                mem_properties,
                total_size,
                self.usage,
                self.properties,
            );
            self.gpu_buffers.push(new_buffer);

            // Считаем диапазон ID для нового буфера
            let start_id = (self.gpu_buffers.len() - 1) as u64 * 256;
            let end_id = start_id + 256;
            self.free_slots.extend(start_id..end_id);
        }

        // 2. Забираем первый свободный слот из пула
        let global_slot_id = self.free_slots.pop().unwrap();

        // Находим, какому физическому буферу и какому локальному индексу он принадлежит
        let buffer_idx = (global_slot_id / 256) as usize;
        let slot_idx = (global_slot_id % 256) as u32;

        let offset = slot_idx as vk::DeviceSize * self.slot_size;

        SlotAllocData {
            buffer: self.gpu_buffers[buffer_idx].buffer,
            offset,
            slot_idx,
            buffer_idx,
            global_slot_id,
        }
    }

    /// Освобождаем слот, возвращая его ID в пул свободных
    pub fn free(&mut self, alloc: SlotAllocData) {
        // Просто пушим глобальный ID обратно в вектор
        self.free_slots.push(alloc.global_slot_id);
    }
}
