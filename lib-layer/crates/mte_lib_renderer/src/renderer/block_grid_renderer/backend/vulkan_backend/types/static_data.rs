use ash::vk;
use mte_macros::vfs_include_bytes;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::{
    TextureArrayImage, VkBuilder
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::indirect_buffer_manager::IndirectBufferManager;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataDL;

pub struct StaticData {
    pub block_properties_buffer: VkBufferDataDL,
    pub block_sampler: vk::Sampler,
    pub texture_array: TextureArrayImage,
    pub texture_view: vk::ImageView,
}

impl StaticData {
    pub fn new(
        device: &ash::Device,
        memory_prop: &vk::PhysicalDeviceMemoryProperties,
        block_properties_len: usize,
        layer_count: u32,
    ) -> Self {
        let block_props_size =
            (std::mem::size_of::<u64>() * block_properties_len) as vk::DeviceSize;

        let block_properties_buffer = VkBufferDataDL::new(
            device,
            memory_prop,
            block_props_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        // 2. Создаем сэмплер для блоков
        let block_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::NEAREST)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .min_lod(0.0)
            .max_lod(32.0);

        let block_sampler = unsafe { device.create_sampler(&block_sampler_info, None).unwrap() };

        // 3. Создаем текстурный массив
        let texture_array = TextureArrayImage::new(device, memory_prop, 16, layer_count);
        let texture_view = VkBuilder::create_texture_array_view(
            device,
            texture_array.image,
            vk::Format::R8G8B8A8_SRGB,
            layer_count,
        );

        Self {
            block_properties_buffer,
            block_sampler,
            texture_array,
            texture_view,
        }
    }

    pub unsafe fn destroy(self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.block_sampler, None);
            device.destroy_image_view(self.texture_view, None);
            self.block_properties_buffer.destroy(device);
        }
    }

    #[inline]
    pub fn get_set_layout(device: &ash::Device) -> ash::vk::DescriptorSetLayout {
        let bindings_set0 = [
            // binding = 0: Textures
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            // binding = 1: Sampler
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            // binding = 2: Block Properties
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings_set0),
                    None,
                )
                .unwrap()
        }
    }

    pub unsafe fn upload(
        device: &ash::Device,
        graphics_queue: vk::Queue,
        command_pool: vk::CommandPool,
        buffer_manager: &mut IndirectBufferManager, // Твой менеджер буферов
        dst_block_properties_buffer: vk::Buffer,    // Целевой буфер свойств блоков на GPU
        dst_texture_image: vk::Image,               // Целевая текстура на GPU
        block_properties_data: &[u8],               // Массив свойств блоков из Rust
        texture_resolution: u32,                    // Например, 16
        layer_count: u32,
    ) {
        let image_bytes =
            vfs_include_bytes!("workspace://game-layer/crates/minecraft/content/000001");

        let staging_buffer = &mut buffer_manager.cpu_staging_buffer;

        // --- ШАГ А: Загружаем свойства блоков в стейджинг ---
        let props_src_offset = staging_buffer.len() as vk::DeviceSize;
        staging_buffer.extend_from_slice(block_properties_data);

        // Выравниваем хвост перед записью текстуры, чтобы адрес делился на 4
        while staging_buffer.len() % 4 != 0 {
            staging_buffer.push(0);
        }

        // --- ШАГ Б: Загружаем пиксели текстуры в стейджинг (сразу следом) ---
        let texture_src_offset = staging_buffer.len() as vk::DeviceSize;
        staging_buffer.extend_from_slice(image_bytes);

        let gpu_staging_buffer_vk = buffer_manager.gpu_staging_buffer.buffer;

        // Переносим данные из cpu_staging_buffer (Vec<u8>) в gpu_staging_buffer (Host-Visible память)
        unsafe {
            std::ptr::copy_nonoverlapping(
                buffer_manager.cpu_staging_buffer.as_ptr() as *const std::ffi::c_void,
                buffer_manager.gpu_staging_buffer.mapped_ptr,
                buffer_manager.cpu_staging_buffer.len(),
            );
        }

        // ====================================================================
        // 2. ОТКРЫВАЕМ ОДНОРАЗОВЫЙ КОМАНДНЫЙ БУФЕР ДЛЯ КОПИРОВАНИЯ НА GPU
        // ====================================================================
        let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd = unsafe { device.allocate_command_buffers(&cmd_alloc_info).unwrap() };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(cmd[0], &begin_info).unwrap() };

        // ====================================================================
        // 3. КОПИРОВАНИЕ БУФЕРА СВОЙСТВ БЛОКОВ
        // ====================================================================
        let props_copy_region = vk::BufferCopy::default()
            .src_offset(props_src_offset)
            .dst_offset(0)
            .size(block_properties_data.len() as vk::DeviceSize);

        unsafe {
            device.cmd_copy_buffer(
                cmd[0],
                gpu_staging_buffer_vk,
                dst_block_properties_buffer,
                &[props_copy_region],
            )
        };

        // Барьер памяти для буфера свойств блоков (Буфер должен перейти из TRANSFER_WRITE в SHADER_READ)
        let buffer_barrier = vk::BufferMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .buffer(dst_block_properties_buffer)
            .offset(0)
            .size(vk::WHOLE_SIZE);

        // ====================================================================
        // 4. КОПИРОВАНИЕ МАССИВА ТЕКСТУР (IMAGE)
        // ====================================================================

        // Переводим текстуру из UNDEFINED в статус TRANSFER_DST_OPTIMAL (цель копирования)
        let barrier_to_transfer = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .image(dst_texture_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count,
            });

        // Применяем барьер для текстуры и свойства буфера одновременно
        unsafe {
            device.cmd_pipeline_barrier(
                cmd[0],
                vk::PipelineStageFlags::TOP_OF_PIPE | vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_to_transfer],
            )
        };

        // Описываем слои для копирования в texture2DArray
        let image_subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(layer_count);

        let image_copy_region = vk::BufferImageCopy::default()
            .buffer_offset(texture_src_offset) // Смещение пикселей внутри твоего стейджинга
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(image_subresource)
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: texture_resolution,
                height: texture_resolution,
                depth: 1,
            });

        unsafe {
            device.cmd_copy_buffer_to_image(
                cmd[0],
                gpu_staging_buffer_vk,
                dst_texture_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[image_copy_region],
            )
        };

        // Переводим текстуру в финальный лейаут SHADER_READ_ONLY_OPTIMAL для чтения во фрагментном шейдере
        let barrier_to_shader = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ)
            .image(dst_texture_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count,
            });

        // Финальная синхронизация: открываем данные для шейдеров (Свойства блоков для VERTEX, текстуры для FRAGMENT)
        unsafe {
            device.cmd_pipeline_barrier(
                cmd[0],
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_SHADER | vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[buffer_barrier],    // Защищаем буфер свойств
                &[barrier_to_shader], // Защищаем текстуру
            )
        };

        unsafe { device.end_command_buffer(cmd[0]).unwrap() };

        // Отправляем в очередь и синхронно ждем окончания выполнения на GPU
        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe { device.create_fence(&fence_info, None).unwrap() };
        unsafe {
            device
                .queue_submit(
                    graphics_queue,
                    &[vk::SubmitInfo::default().command_buffers(&[cmd[0]])],
                    fence,
                )
                .unwrap();
            device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
        };

        // Освобождаем временные ресурсы одноразовой команды
        unsafe {
            device.destroy_fence(fence, None);
            device.free_command_buffers(command_pool, &[cmd[0]]);
        }

        // Очищаем стейджинг буфер на CPU, он готов к работе с чанками в игровом цикле
        buffer_manager.cpu_staging_buffer.clear();
    }
}
