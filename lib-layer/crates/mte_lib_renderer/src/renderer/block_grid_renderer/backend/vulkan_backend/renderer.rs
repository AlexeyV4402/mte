use std::collections::HashSet;
use std::ffi::CStr;
use std::sync::Arc;

use ash::vk::*;
use ash::{Entry, vk};
use lib_core::alloc_helper::ConstPageAllocHelper;
use mte_macros::{vfs_include_bytes, vfs_include_vk_shader};
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use winit::dpi::PhysicalSize;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::{
    DepthBuffer, SyncObjects, TextureArrayImage, VkBuilder, create_command_pool_and_sync, upload_static_resources
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::indirect_buffer_manager::{
    ChunkGpuHandle, IndirectBufferManager
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_manager;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataHV;
use crate::renderer::block_grid_renderer::consts::{
    GLOBAL_INDEX_BUFFER_CAPACITY, GLOBAL_VERTEX_BUFFER_CAPACITY
};
use crate::renderer::block_grid_renderer::inditect_buffer_manager;
use crate::renderer::block_grid_renderer::render_objects::camera::WorldCameraUniform;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use crate::renderer::block_grid_renderer::renderer::RendererCreateArgs;
use crate::renderer::block_grid_renderer::types::BlockVertex;

pub struct VkBackend {
    pub entry: Entry,
    pub phys_dev: ash::vk::PhysicalDevice,
    pub instance: ash::Instance,
    pub memory_prop: PhysicalDeviceMemoryProperties,
    pub surface: ash::vk::SurfaceKHR,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,

    // Свопчейн и поверхности
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub depth_buffer: DepthBuffer,

    // Конвейер рендеринга
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub pipeline_layout: vk::PipelineLayout,
    pub graphics_pipeline: vk::Pipeline,

    pub cmd_pool: vk::CommandPool,
    pub sync_objects: SyncObjects,
    pub command_buffers: Vec<vk::CommandBuffer>,

    // Синхронизация кадров
    pub current_frame: usize,
    pub image_index: u32, // Индекс текущей картинки свопчейна, полученный в методе frame()

    // --- НАШ МЕНЕДЖЕР БУФЕРОВ ---
    pub buffer_manager: IndirectBufferManager,

    // Буфер для камеры на GPU (HOST_VISIBLE | HOST_COHERENT) и замаппленный указатель на него
    pub camera_buffer: vk::Buffer,
    pub camera_memory: vk::DeviceMemory,
    pub camera_mapped_ptr: *mut std::ffi::c_void,

    pub class_sampler: vk::Sampler,
    pub global_descriptor_pool: vk::DescriptorPool,
    pub set0_blocks: vk::DescriptorSet,
    pub set1_camera: vk::DescriptorSet,
    pub set2_vectors: vk::DescriptorSet,
    pub block_properties_buffer: vk::Buffer,
    pub block_properties_memory: vk::DeviceMemory,

    pub test_slot_idx: u64,
    pub last_frame: std::time::Instant,
}

impl VkBackend {
    pub fn new(
        window: Arc<winit::window::Window>,
        renderer_create_args: RendererCreateArgs,
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
        let entry = unsafe { Entry::load().unwrap() };

        let version = unsafe {
            entry
                .try_enumerate_instance_version()
                .expect("Error enumerate instance version")
        };

        let api_version = match version {
            Some(version) => version,
            None => API_VERSION_1_0,
        };

        let app_info = ApplicationInfo::default()
            .application_name(c"Minecraft")
            .engine_name(c"MTE")
            .engine_version(0)
            .application_version(0)
            .api_version(api_version);

        let instance_required_extensions = [
            CStr::from_bytes_with_nul(b"VK_KHR_surface\0")
                .unwrap()
                .as_ptr(),
            CStr::from_bytes_with_nul(b"VK_KHR_wayland_surface\0")
                .unwrap()
                .as_ptr(),
        ];

        let layer_names = [CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()];
        let layers_pointers: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let instance_info = InstanceCreateInfo::default()
            .enabled_extension_names(&instance_required_extensions)
            .enabled_layer_names(&layers_pointers)
            .application_info(&app_info);

        let instance = unsafe {
            entry
                .create_instance(&instance_info, None)
                .map_err(|e| format!("Error create insatnce with errror: {}", e))
                .unwrap()
        };

        // let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let surface: ash::vk::SurfaceKHR = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle().unwrap().into(),
                window.window_handle().unwrap().into(),
                None, // Аллокатор памяти Vulkan (обычно None)
            )
            .expect("Не удалось создать VkSurfaceKHR для твоего окна ОС!")
        };

        let phys_dev_required_extensions =
            [CStr::from_bytes_with_nul(b"VK_KHR_swapchain\0").unwrap()];
        let phys_dev_required_extensions_ptrs = [CStr::from_bytes_with_nul(b"VK_KHR_swapchain\0")
            .unwrap()
            .as_ptr()];

        let phys_dev = VkBuilder::find_device(&instance, phys_dev_required_extensions);

        let memory_prop = unsafe { instance.get_physical_device_memory_properties(phys_dev) };
        let queue_family_prop =
            unsafe { instance.get_physical_device_queue_family_properties(phys_dev) };
        let phys_prop = unsafe { instance.get_physical_device_properties(phys_dev) };

        if phys_prop.limits.max_push_constants_size < 160 {
            panic!("Видеоадаптер не соответствует требованиям");
        }

        struct QueueFamilyInfo {
            queue_family_index: usize,
            queue_prop: QueueFamilyProperties,
        }

        let mut queue_infos = vec![];

        for (index, i) in queue_family_prop.iter().enumerate() {
            queue_infos.push(QueueFamilyInfo {
                queue_family_index: index,
                queue_prop: *i,
            });
        }

        let priority = [1.0f32];
        let mut queue_family_infos = vec![];

        for i in queue_infos {
            let device_queue_info = DeviceQueueCreateInfo::default()
                .queue_family_index(i.queue_family_index as u32)
                .queue_priorities(&priority);

            queue_family_infos.push(device_queue_info)
        }

        let features = PhysicalDeviceFeatures::default()
            .multi_draw_indirect(true)
            .fill_mode_non_solid(true);

        let device_info = DeviceCreateInfo::default()
            .enabled_features(&features)
            .queue_create_infos(&queue_family_infos)
            .enabled_extension_names(&phys_dev_required_extensions_ptrs);

        let device = unsafe {
            instance
                .create_device(phys_dev, &device_info, None)
                .expect("Error create device")
        };

        let main_graphics_queue = unsafe { device.get_device_queue(0, 0) };

        let surface_format = vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        };

        let (swapchain, swapchain_image_views, swapchain_extent, swapchain_loader) =
            VkBuilder::create_swapchain(
                &entry,
                &instance,
                &device,
                phys_dev,
                surface,
                surface_format,
                window.inner_size(),
            );

        let (cmd_pool, sync_objects) = create_command_pool_and_sync(&device, 0, 2);

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY) // PRIMARY означает, что этот буфер можно отправить напрямую в видеокарту
            .command_buffer_count(2); // Нам нужно ровно 2 штуки (для Double Buffering)

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&alloc_info)
                .expect("Не удалось аллоцировать Command Buffers")
        };

        let depth_buffer =
            DepthBuffer::new(&instance, &device, phys_dev, &memory_prop, swapchain_extent);

        let mut framebuffers = Vec::new();
        let render_pass =
            VkBuilder::create_render_pass(&device, surface_format, depth_buffer.format);

        for &swapchain_view in &swapchain_image_views {
            // Порядок вложений (attachments) должен СТРОГО совпадать с порядком в вашем RenderPass!
            // Обычно: сначала Цвет (0), потом Глубина (1)
            let attachments = [
                swapchain_view,    // Цветной холст свопчейна
                depth_buffer.view, // Общий холст глубины
            ];

            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass) // Конвейер должен знать, под какую схему рендера создается фреймбуфер
                .attachments(&attachments)
                .width(swapchain_extent.width) // Ширина под размер окна
                .height(swapchain_extent.height) // Высота под размер окна
                .layers(1); // Для обычного 3D-экрана всегда 1 слой

            let framebuffer = unsafe {
                device
                    .create_framebuffer(&framebuffer_info, None)
                    .expect("Не удалось создать Framebuffer")
            };
            framebuffers.push(framebuffer);
        }

        let chunks_vert_shader_module = VkBuilder::create_shader_module(&device, chunks_vert);
        let chunks_frag_shader_module = VkBuilder::create_shader_module(&device, chunks_frag);
        // let hand_vert_shader_module = VkBuilder::create_shader_module(&device, hand_vert);
        // let hand_frag_shader_module = VkBuilder::create_shader_module(&device, hand_frag);

        let block_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::NEAREST)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .min_lod(0.0)
            .max_lod(32.0)
            .anisotropy_enable(false)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK);

        let block_sampler = unsafe {
            device
                .create_sampler(&block_sampler_info, None)
                .expect("Не удалось создать сэмплер для блоков")
        };

        // ====================================================================
        // ШАГ 2: Создаем DescriptorSetLayout для каждого Set (Лейауты бинд-групп)
        // ====================================================================

        // --- LAYOUT SET 0: Статические данные блоков ---
        let bindings_set0 = [
            // binding = 0: Массив текстур во Фрагментном шейдере
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            // binding = 1: Сэмплер во Фрагментном шейдере
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
            // binding = 2: Свойства блоков в Вершинном шейдере
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
        ];
        let layout_set0_blocks = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings_set0),
                    None,
                )
                .unwrap()
        };

        // --- LAYOUT SET 1: Данные камеры ---
        let binding_set1 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set1_camera = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set1)),
                    None,
                )
                .unwrap()
        };

        // --- LAYOUT SET 2: Вектора смещения чанков ---
        let binding_set2 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set2_vectors = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set2)),
                    None,
                )
                .unwrap()
        };

        // ====================================================================
        // ШАГ 3: Создаем ОДИН ГЛОБАЛЬНЫЙ ПУЛ на весь рендерер
        // ====================================================================
        let pool_sizes = [
            // Для текстуры кадра (set 0 binding 0)
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1),
            // Для сэмплера (set 0 binding 1)
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(1),
            // Для двух Storage буферов (свойства блоков set 0 и вектора set 2)
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(2),
            // Для Uniform буфера камеры (set 1)
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(2),
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(3) // Всего нарезаем ровно 3 отдельных набора (сета)
            .pool_sizes(&pool_sizes);

        let global_descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .expect("Не удалось создать глобальный DescriptorPool")
        };

        // ====================================================================
        // ШАГ 4: Нарезаем дескрипторные сеты (Бинд-группы)
        // ====================================================================
        let set_layouts = [layout_set0_blocks, layout_set1_camera, layout_set2_vectors];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(global_descriptor_pool)
            .set_layouts(&set_layouts);

        let descriptor_sets = unsafe {
            device
                .allocate_descriptor_sets(&alloc_info)
                .expect("Не удалось выделить дескрипторные сеты")
        };

        let set0_blocks = descriptor_sets[0];
        let set1_camera = descriptor_sets[1];
        let set2_vectors = descriptor_sets[2];

        // ====================================================================
        // ШАГ 5: Создаем буфер под свойства блоков (Вместо wgpu буфера)
        // ====================================================================
        let block_props_size = (std::mem::size_of::<u64>()
            * renderer_create_args.block_properties.len())
            as vk::DeviceSize;
        let block_props_buffer_info = vk::BufferCreateInfo::default()
            .size(block_props_size)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let block_properties_buffer = unsafe {
            device
                .create_buffer(&block_props_buffer_info, None)
                .unwrap()
        };
        let block_props_mem_reqs =
            unsafe { device.get_buffer_memory_requirements(block_properties_buffer) };

        // Ищем DEVICE_LOCAL тип памяти
        let mut block_props_mem_type_index = 0;
        for i in 0..memory_prop.memory_type_count {
            if (block_props_mem_reqs.memory_type_bits & (1 << i)) != 0
                && memory_prop.memory_types[i as usize]
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            {
                block_props_mem_type_index = i;
                break;
            }
        }

        let block_props_alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(block_props_mem_reqs.size)
            .memory_type_index(block_props_mem_type_index);

        let block_properties_memory = unsafe {
            device
                .allocate_memory(&block_props_alloc_info, None)
                .unwrap()
        };
        unsafe {
            device
                .bind_buffer_memory(block_properties_buffer, block_properties_memory, 0)
                .unwrap()
        };

        // ====================================================================
        // ШАГ 6: Вызываем update_descriptor_sets для связки ресурсов
        // ====================================================================

        let mut buffer_manager = IndirectBufferManager::new(&device, &memory_prop);

        let camera_buffer = VkBufferDataHV::new(
            &device,
            &memory_prop,
            size_of::<WorldCameraUniform>() as u64,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );

        let (camera_buffer, camera_memory, camera_mapped_ptr) = (
            camera_buffer.buffer,
            camera_buffer.mem,
            camera_buffer.mapped_ptr,
        );

        let texture_array =
            TextureArrayImage::new(&device, &memory_prop, 16, renderer_create_args.layer_count);

        let texture_view = VkBuilder::create_texture_array_view(
            &device,
            texture_array.image,
            vk::Format::R8G8B8A8_SRGB,
            renderer_create_args.layer_count,
        );

        // Допустим, у тебя уже есть texture_view от загруженного массива текстур
        let texture_image_info = vk::DescriptorImageInfo::default()
            .image_view(texture_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let sampler_image_info = vk::DescriptorImageInfo::default().sampler(block_sampler);

        let block_props_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(block_properties_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let camera_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(camera_buffer) // Из твоего метода создания буфера камеры
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let vectors_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(buffer_manager.vector_buffer.gpu_buffers[0].buffer) // Из твоего менеджера буферов
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let writes = [
            // --- SET 0 ---
            vk::WriteDescriptorSet::default()
                .dst_set(set0_blocks)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(std::slice::from_ref(&texture_image_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(set0_blocks)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(std::slice::from_ref(&sampler_image_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(set0_blocks)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(std::slice::from_ref(&block_props_buffer_info)),
            // --- SET 1 ---
            vk::WriteDescriptorSet::default()
                .dst_set(set1_camera)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&camera_buffer_info)),
            // --- SET 2 ---
            vk::WriteDescriptorSet::default()
                .dst_set(set2_vectors)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&vectors_buffer_info)),
        ];
        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }

        let pipeline_layout = VkBuilder::create_pipeline_layout(&device, &set_layouts);

        let vertex_binding_descriptions = [
            vk::VertexInputBindingDescription::default()
                .binding(0) // Индекс буфера (обычно 0)
                .stride(std::mem::size_of::<BlockVertex>() as u32) // Шаг в байтах (20 байт)
                .input_rate(vk::VertexInputRate::VERTEX), // Шагаем по вершинам (не по инстансам)
        ];

        // 2. Описываем аттрибуты (поля внутри структуры)
        let vertex_attribute_descriptions = [
            // layout(location = 0) in vec3 position;
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0) // Должно совпадать с location в GLSL шейдере!
                .format(vk::Format::R32_UINT) // vec3 из f32
                .offset(0), // Начинается с самого начала структуры
        ];

        let graphics_pipeline = VkBuilder::create_graphics_pipeline(
            &device,
            pipeline_layout,
            render_pass,
            chunks_vert_shader_module,
            chunks_frag_shader_module,
            vertex_binding_descriptions.as_slice(),
            vertex_attribute_descriptions.as_slice(),
        );

        unsafe {
            upload_static_resources(
                &device,
                main_graphics_queue,
                cmd_pool,
                &mut buffer_manager,
                block_properties_buffer,
                texture_array.image,
                renderer_create_args.block_properties,
                16,
                renderer_create_args.layer_count,
            )
        };

        Self {
            entry,
            phys_dev,
            instance,
            memory_prop,
            surface,
            device,
            graphics_queue: main_graphics_queue,
            swapchain_loader,
            swapchain,
            swapchain_extent,
            swapchain_image_views,
            depth_buffer,
            render_pass,
            framebuffers,
            pipeline_layout,
            graphics_pipeline,
            cmd_pool,
            sync_objects,
            command_buffers,
            current_frame: 0,
            image_index: 0,
            buffer_manager,
            camera_buffer,
            camera_memory,
            camera_mapped_ptr,
            class_sampler: block_sampler,
            global_descriptor_pool,
            set0_blocks,
            set1_camera,
            set2_vectors,
            block_properties_buffer,
            block_properties_memory,
            test_slot_idx: Default::default(),
            last_frame: std::time::Instant::now(),
        }
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        // Если окно свернули в 0 (например, минимизировали), ничего не пересоздаем, иначе Vulkan упадет
        if new_width == 0 || new_height == 0 {
            return;
        }

        unsafe {
            // 1. Ждем, пока GPU полностью закончит все текущие операции отрисовки.
            // Нельзя удалять текстуры, которые видеокарта прямо сейчас красит!
            self.device.device_wait_idle().unwrap();

            // ====================================================================
            // ШАГ 1: ОЧИСТКА СТАРЫХ РЕСУРСОВ
            // ====================================================================

            // Уничтожаем старые фреймбуферы (они были привязаны к старому разрешению холстов)
            for &fb in &self.framebuffers {
                self.device.destroy_framebuffer(fb, None);
            }
            self.framebuffers.clear();

            // Полностью уничтожаем старый буфер глубины и освобождаем его VRAM память
            self.device.destroy_image_view(self.depth_buffer.view, None);
            self.device.destroy_image(self.depth_buffer.image, None);
            self.device.free_memory(self.depth_buffer.memory, None);

            // Уничтожаем старые ImageView для картинок свопчейна
            for &view in &self.swapchain_image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain_image_views.clear();

            // Запоминаем старый свопчейн, чтобы передать его как "old_swapchain" для бесшовного перехода
            let old_swapchain = self.swapchain;

            // ====================================================================
            // ШАГ 2: ПЕРЕСОЗДАНИЕ СВОПЧЕЙНА С НОВЫМ РАЗРЕШЕНИЕМ
            // ====================================================================
            let new_size = winit::dpi::PhysicalSize::new(new_width, new_height);

            // Задаем жесткий формат, который мы использовали при первой инициализации
            let surface_format = vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_SRGB,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };

            // Вызываем твою функцию создания свопчейна.
            // ВАЖНО: Если твой `VkBuilder::create_swapchain` не умеет принимать `old_swapchain`,
            // то старый свопчейн нужно просто удалить через `destroy_swapchain` до этого шага.
            // Предположим, что мы его удаляем для простоты, если конструктор не оптимизирован:
            self.swapchain_loader.destroy_swapchain(old_swapchain, None);

            let (new_swapchain, new_views, new_extent, _) = VkBuilder::create_swapchain(
                &self.entry, // Если они сохранены в Renderer, берем их, либо передай аргументами
                &self.instance,
                &self.device,
                self.phys_dev,
                self.surface,
                surface_format,
                new_size,
            );

            self.swapchain = new_swapchain;
            self.swapchain_image_views = new_views;
            self.swapchain_extent = new_extent;

            // ====================================================================
            // ШАГ 3: СОЗДАНИЕ НОВОГО БУФЕРА ГЛУБИНЫ ПОД НОВЫЙ РАЗМЕР ОКНА
            // ====================================================================
            self.depth_buffer = DepthBuffer::new(
                &self.instance,
                &self.device,
                self.phys_dev,
                &self.memory_prop,
                self.swapchain_extent,
            );

            // ====================================================================
            // ШАГ 4: ПЕРЕСОЗДАНИЕ ФРЕЙМБУФЕРОВ
            // ====================================================================
            // Снова связываем новые ImageView свопчейна и новый общий буфер глубины
            for &swapchain_view in &self.swapchain_image_views {
                let attachments = [
                    swapchain_view,         // Порядок СТРОГО как в RenderPass (Цвет — 0)
                    self.depth_buffer.view, // Глубина — 1
                ];

                let framebuffer_info = vk::FramebufferCreateInfo::default()
                    .render_pass(self.render_pass) // Наш RenderPass пересоздавать НЕ НАДО, его схема не изменилась
                    .attachments(&attachments)
                    .width(self.swapchain_extent.width)
                    .height(self.swapchain_extent.height)
                    .layers(1);

                let framebuffer = self
                    .device
                    .create_framebuffer(&framebuffer_info, None)
                    .expect("Не удалось воссоздать Framebuffer при ресайзе");

                self.framebuffers.push(framebuffer);
            }
        }
    }

    pub fn load_chunk(
        &mut self,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
    ) -> Option<ChunkGpuHandle> {
        let current_cmd = self.command_buffers[self.current_frame];

        let handle = self.buffer_manager.load_chunk(
            primitive,
            vector,
            &self.device,
            &self.memory_prop,
            current_cmd, // Передаем общий буфер кадра вместо одноразового мусора!
        );

        handle
    }

    pub fn begin_frame(&mut self) {
        let frame = self.current_frame;
        let cmd = self.command_buffers[frame];

        unsafe {
            self.device
                .wait_for_fences(&[self.sync_objects.in_flight_fences[frame]], true, u64::MAX)
                .unwrap();

            // Запрашиваем картинку. Свопчейн засигналит семафор,
            // завязанный НА ИНДЕКС КАРТИНКИ, которую он выдает!
            // Но чтобы узнать этот индекс, нам сначала нужен временный семафор.
            // Поэтому на этапе acquire_next_image мы используем семафор по индексу frame (0 или 1):
            let (image_index, _) = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    self.sync_objects.image_available_semaphores[frame], // Временно используем frame
                    vk::Fence::null(),
                )
                .unwrap();

            self.image_index = image_index;

            self.device
                .reset_fences(&[self.sync_objects.in_flight_fences[frame]])
                .unwrap();
            self.device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .unwrap();

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(cmd, &begin_info).unwrap();
        }
    }

    pub fn unload_chunk(&mut self, handle: ChunkGpuHandle) {
        self.buffer_manager.unload_chunk(handle);
    }

    pub fn end_frame(&mut self) -> std::result::Result<(), ash::vk::Result> {
        let frame = self.current_frame;
        let img_idx = self.image_index as usize;
        let cmd = self.command_buffers[frame];

        unsafe {
            self.buffer_manager.prepare_buffers(&self.device, cmd);

            let indirect_barrier = vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::INDIRECT_COMMAND_READ) // Защищаем чтение indirect-команд
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(
                    self.buffer_manager
                        .indirect_buffer
                        .gpu_indexed_indirect_buffer
                        .buffer,
                )
                .offset(0)
                .size(vk::WHOLE_SIZE);

            // 2. Барьер для Вершинного буфера
            let vertex_barrier = vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ) // Защищаем чтение вершин геометрическим блоком GPU
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer_manager.vertex_buffer.gpu_buffers[0].buffer) // Достаем VkBuffer твоего PageBuffer
                .offset(0)
                .size(vk::WHOLE_SIZE);

            // 3. Барьер для Индексного буфера
            let index_barrier = vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::INDEX_READ) // Защищаем чтение индексов кубов
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer_manager.index_buffer.gpu_buffers[0].buffer) // Достаем VkBuffer твоего PageBuffer
                .offset(0)
                .size(vk::WHOLE_SIZE);

            let vector_barrier = vk::BufferMemoryBarrier::default()
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ) // Защищаем чтение индексов кубов
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .buffer(self.buffer_manager.matrix_buffer.gpu_buffers[0].buffer) // Достаем VkBuffer твоего PageBuffer
                .offset(0)
                .size(vk::WHOLE_SIZE);

            // Собираем их в массив
            let buffer_barriers = [
                indirect_barrier,
                vertex_barrier,
                index_barrier,
                vector_barrier,
            ];

            self.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER, // Стадия-источник (копирование)
                vk::PipelineStageFlags::VERTEX_INPUT
                    | vk::PipelineStageFlags::DRAW_INDIRECT
                    | vk::PipelineStageFlags::VERTEX_SHADER, // Стадии-потребители (ввод геометрии и индирект)
                vk::DependencyFlags::empty(),
                &[],
                &buffer_barriers,
                &[],
            );

            // ====================================================================
            // ШАГ 2: НАЧАЛО RENDER PASS (ОЧИСТКА ЭКРАНА И ГЛУБИНЫ)
            // ====================================================================
            let clear_values = [
                // Индекс 0: Очищаем цветной холст свопчейна в цвет неба
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                // Индекс 1: Очищаем буфер глубины в 1.0 (самая дальняя точка)
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[self.image_index as usize]) // Наш фреймбуфер для текущего кадра свопчейна
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                })
                .clear_values(&clear_values);

            // Входим в Render Pass
            self.device
                .cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

            // ====================================================================
            // ШАГ 3: ДИНАМИЧЕСКИЕ viewport И scissor (ПОД РАЗМЕР ОКНА)
            // ====================================================================
            let viewport = vk::Viewport::default()
                .x(0.0)
                .y(0.0)
                .width(self.swapchain_extent.width as f32)
                .height(self.swapchain_extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0);

            let scissor = vk::Rect2D::default().extent(self.swapchain_extent);

            self.device.cmd_set_viewport(cmd, 0, &[viewport]);
            self.device.cmd_set_scissor(cmd, 0, &[scissor]);

            // ====================================================================
            // ШАГ 4: ВКЛЮЧАЕМ КОНВЕЙЕР И ДЕСКРИПТОРЫ
            // ====================================================================
            // Активируем запеченный графический конвейер блоков
            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );

            // Собираем массив наших бинд-групп (Set 0, Set 1, Set 2)
            let sets = [self.set0_blocks, self.set1_camera, self.set2_vectors];

            // Подключаем все дескрипторы разом к конвейеру
            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0, // Начинаем привязку строго с set = 0
                &sets,
                &[],
            );

            // ====================================================================
            // ШАГ 5: ЖЕЛЕЗНЫЙ МУЛЬТИ-ДРОУ ВЫЗОВ (ОТРИСОВКА МИРА)
            // ====================================================================
            self.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.buffer_manager.vertex_buffer.gpu_buffers[0].buffer],
                &[0],
            );

            // Привязываем гигантский индексный буфер (под u32 индексы)
            self.device.cmd_bind_index_buffer(
                cmd,
                self.buffer_manager.index_buffer.gpu_buffers[0].buffer,
                0,
                vk::IndexType::UINT32,
            );

                self.device.cmd_draw_indexed_indirect(
                    cmd,
                    self.buffer_manager
                        .indirect_buffer
                        .gpu_indexed_indirect_buffer
                        .buffer,
                    0,
                    self.buffer_manager.indirect_buffer.cpu_indexed_indirect_buffer.len() as u32,
                    std::mem::size_of::<vk::DrawIndexedIndirectCommand>() as u32,
                );

            // Выходим из Render Pass и закрываем «блокнот» команд кадра
            self.device.cmd_end_render_pass(cmd);
            self.device.end_command_buffer(cmd).unwrap();

            // ====================================================================
            // ШАГ 6: ОТПРАВКА НА ИСПОЛНЕНИЕ В GPU (SUBMIT)
            // ====================================================================
            let wait_semaphores = [self.sync_objects.image_available_semaphores[frame]];

            // СИГНАЛ-семафор жестко привязываем К ИНДЕКСУ КАРТИНКИ!
            let signal_semaphores = [self.sync_objects.render_finished_semaphores[img_idx]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let submit_cmds = [cmd];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&submit_cmds)
                .signal_semaphores(&signal_semaphores);

            self.device
                .queue_submit(
                    self.graphics_queue,
                    &[submit_info],
                    self.sync_objects.in_flight_fences[frame],
                )
                .unwrap();

            // Монитор будет ждать ИМЕННО тот семафор, который привязан к этой картинке
            let present_wait_semaphores = [self.sync_objects.render_finished_semaphores[img_idx]];
            let present_swapchains = [self.swapchain];
            let present_image_indices = [self.image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&present_wait_semaphores)
                .swapchains(&present_swapchains)
                .image_indices(&present_image_indices);

            self.swapchain_loader
                .queue_present(self.graphics_queue, &present_info)
                .unwrap();
            // self.last_frame = std::time::Instant::now();
            // println!("slot_idx: {};Запуск очереди", self.test_slot_idx,);
        }

        unsafe {
            self.device
                .wait_for_fences(
                    &[self.sync_objects.in_flight_fences[self.current_frame]],
                    true,
                    u64::MAX,
                )
                .unwrap();
        }

        // println!(
        //     "slot_idx: {};Завершение очереди: {}",
        //     self.test_slot_idx,
        //     self.last_frame.elapsed().as_micros()
        // );
        // self.test_slot_idx = (self.test_slot_idx + 1) % 257;

        self.current_frame = (self.current_frame + 1) % 2;

        std::result::Result::Ok(())
    }

    pub fn update_camera(&mut self, pass_1_camera_uniform: WorldCameraUniform) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                &pass_1_camera_uniform as *const WorldCameraUniform,
                self.camera_mapped_ptr as *mut WorldCameraUniform,
                1,
            );
        }
    }
}
