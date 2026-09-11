use std::collections::HashSet;
use std::ffi::CStr;

use ash::vk::*;
use ash::{Entry, vk};
use mte_macros::vfs_include_bytes;
use winit::dpi::PhysicalSize;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::indirect_buffer_manager::IndirectBufferManager;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::find_memory_type;
use crate::renderer::block_grid_renderer::types::BlockVertex;

pub struct VkBuilder {}

impl VkBuilder {
    pub fn find_device<const N: usize>(
        instance: &ash::Instance,
        required_device_extensions: [&CStr; N],
    ) -> PhysicalDevice {
        let phys_devs = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Error enumerate Physical Devices")
        };

        let mut chosen_phys_dev = None;

        for &device in &phys_devs {
            let prop = unsafe { instance.get_physical_device_properties(device) };

            let available_extensions = unsafe {
                instance
                    .enumerate_device_extension_properties(device)
                    .expect("Failed to get device extensions")
            };

            let available_ext_names: HashSet<&CStr> = available_extensions
                .iter()
                .map(|ext| unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) })
                .collect();

            let supports_required_extensions = required_device_extensions
                .iter()
                .all(|ext| available_ext_names.contains(ext));

            if supports_required_extensions {
                if prop.device_type == PhysicalDeviceType::DISCRETE_GPU {
                    chosen_phys_dev = Some(device);
                    break;
                }
                if chosen_phys_dev.is_none() {
                    chosen_phys_dev = Some(device);
                }
            }
        }

        chosen_phys_dev
            .expect("Ни одна видеокарта не поддерживает требуемые расширения (VK_KHR_swapchain)!")
    }

    pub fn create_shader_module(device: &ash::Device, bytes: &'static [u8]) -> vk::ShaderModule {
        let mut spirv_u32 = vec![0u32; bytes.len() / 4];

        // Безопасно и быстро копируем байты в выровненный вектор на CPU
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                spirv_u32.as_mut_ptr() as *mut u8,
                bytes.len(),
            );
        }

        // let spirv_u32: &[u32] = bytemuck::cast_slice(bytes);
        let create_info = vk::ShaderModuleCreateInfo::default().code(&spirv_u32); // Передаем &[u32]
        unsafe { device.create_shader_module(&create_info, None) }
            .expect("Не удалось создать ShaderModule")
    }

    pub fn create_pipeline_layout(
        device: &ash::Device,
        layouts: &[DescriptorSetLayout],
    ) -> vk::PipelineLayout {
        let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts);

        unsafe {
            device
                .create_pipeline_layout(&create_info, None)
                .expect("Не удалось создать PipelineLayout")
        }
    }
    
    pub fn create_pipeline_layout_with_push_const_range(
        device: &ash::Device,
        layouts: &[DescriptorSetLayout],
        size: u32
    ) -> vk::PipelineLayout {
        // 64 байта под mat4 (матрица камеры)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(size);
        let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(layouts) 
        .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        unsafe {
            device
                .create_pipeline_layout(&create_info, None)
                .expect("Не удалось создать PipelineLayout")
        }
    }

    pub fn create_graphics_pipeline(
        device: &ash::Device,
        pipeline_layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
        vert_module: vk::ShaderModule,
        frag_module: vk::ShaderModule,
    ) -> vk::Pipeline {
        let vertex_bindings = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<BlockVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)];

        let vertex_attributes = [vk::VertexInputAttributeDescription::default()
            .binding(0)
            .location(0)
            .format(vk::Format::R32_UINT)
            .offset(0)];

        let main_entry = std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap();

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(main_entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(main_entry),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_bindings)
            .vertex_attribute_descriptions(&vertex_attributes);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // Вьюпорт динамический
        let viewport_info = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            // .polygon_mode(vk::PolygonMode::LINE)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            // .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            // .rasterizer_discard_enable(true)
            .depth_bias_enable(false);

        let multisample_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true) // Включаем прозрачность для листвы/воды
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&color_blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_info)
            .color_blend_state(&color_blend_info)
            .dynamic_state(&dynamic_info)
            .depth_stencil_state(&depth_stencil_info)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        unsafe {
            let pipelines = device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .expect("Не удалось скомпилировать графический конвейер");
            pipelines[0]
        }
    }

    pub fn create_framebuffers(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        swapchain_image_views: &[vk::ImageView],
        swapchain_extent: vk::Extent2D,
    ) -> Vec<vk::Framebuffer> {
        swapchain_image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view];
                let create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(swapchain_extent.width)
                    .height(swapchain_extent.height)
                    .layers(1);

                unsafe {
                    device
                        .create_framebuffer(&create_info, None)
                        .expect("Не удалось создать Framebuffer")
                }
            })
            .collect()
    }

    pub fn create_render_pass(
        device: &ash::Device,
        surface_format: vk::SurfaceFormatKHR,
        depth_buffer_format: Format,
    ) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription::default()
            .format(surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR) // Очищаем экран перед каждым кадром
            .store_op(vk::AttachmentStoreOp::STORE) // Сохраняем пиксели для вывода на экран
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR); // Готов к выводу через Swapchain

        // println!("depth_buffer_format: {:?}", depth_buffer_format);
        let depth_attachment = vk::AttachmentDescription::default()
            .format(depth_buffer_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR) // В НАЧАЛЕ КАДРА: Залить буфер глубины значением 1.0 (самая дальняя точка)
            // .load_op(vk::AttachmentLoadOp::DONT_CARE)
            .store_op(vk::AttachmentStoreOp::DONT_CARE) // В КОНЦЕ КАДРА: Нам плевать, сохранятся ли данные глубины (экран их не показывает)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let depth_attachment_ref = vk::AttachmentReference::default()
            .attachment(1) // Индекс в массиве attachments
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref))
            .depth_stencil_attachment(&depth_attachment_ref);

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            );

        let attachments = [color_attachment, depth_attachment];

        let create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dependency));

        unsafe {
            device
                .create_render_pass(&create_info, None)
                .expect("Не удалось создать RenderPass")
        }
    }

    pub fn create_swapchain(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        // capabilities: SurfaceCapabilitiesKHR,
        phys_dev: PhysicalDevice,
        surface: ash::vk::SurfaceKHR,
        surface_format: vk::SurfaceFormatKHR,
        window_size: PhysicalSize<u32>,
    ) -> (
        vk::SwapchainKHR,
        Vec<ImageView>,
        vk::Extent2D,
        ash::khr::swapchain::Device,
    ) {
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(phys_dev, surface)
                .unwrap()
        };

        // 3. Выбираем количество картинок в цепочке (обычно min + 1, чтобы был тройной буфер)
        let mut image_count = capabilities.min_image_count;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);

        let swapchain_extent = if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: window_size.width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: window_size.height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        };
        // 6. Собираем структуру создания Swapchain
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(swapchain_extent)
            .image_array_layers(1) // Для обычного 2D экрана тут всегда 1 (больше нужно для VR/стерео)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // Картинка будет целью для рисования цвета
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE) // Монопольный доступ графической очереди
            .pre_transform(capabilities.current_transform) // Не переворачивать картинку (актуально для мобилок)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE) // Окно не прозрачное, обычный бэкграунд
            .present_mode(vk::PresentModeKHR::FIFO) // Жесткий V-Sync для детерминизма и лока FPS
            .clipped(true); // Разрешить Vulkan не рисовать пиксели, если окно закрыто другим окном ОС

        // // 7. Рожаем сам Swapchain на видеокарте
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Пиздец, Swapchain не создался")
        };

        // // 8. Забираем хэндлы созданных картинок из VRAM
        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };

        let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());

        // // 2. В цикле проходим по каждой картинке из Swapchain
        for &image in &swapchain_images {
            // Описываем компоненты цвета (смешивание каналов).
            // Нам не нужно менять каналы местами (например, делать инверсию),
            // поэтому ставим IDENTITY (оставить как есть).
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR) // Картинка содержит цвет (не глубину/трафарет)
                .base_mip_level(0) // Mip-уровни не используем для экрана (это для текстур)
                .level_count(1)
                .base_array_layer(0) // Слои не используем (это не VR и не 3D-текстура)
                .layer_count(1);

            // Собираем структуру создания ImageView
            let view_create_info = vk::ImageViewCreateInfo::default()
                .image(image) // Привязываем к конкретной сырой картинке из VRAM
                .view_type(vk::ImageViewType::TYPE_2D) // Четко говорим: это обычная двухмерная картинка
                .format(vk::Format::B8G8R8A8_SRGB) // Формат обязан строго совпадать с форматом Swapchain!
                .subresource_range(subresource_range);

            // Рожаем ImageView на видеокарте
            let image_view = unsafe {
                device
                    .create_image_view(&view_create_info, None)
                    .expect("Не удалось создать VkImageView для кадра")
            };

            // Сохраняем хэндл в наш массив
            swapchain_image_views.push(image_view);
        }

        (
            swapchain,
            swapchain_image_views,
            swapchain_extent,
            swapchain_loader,
        )
    }

    fn find_depth_format(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
    ) -> vk::Format {
        let candidates = [
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];

        for &format in &candidates {
            let props =
                unsafe { instance.get_physical_device_format_properties(physical_device, format) };

            // Нам нужно, чтобы формат поддерживал использование в качестве Depth Stencil Attachment
            if props
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            {
                return format;
            }
        }

        panic!("Не удалось найти поддерживаемый формат глубины!");
    }

    pub fn create_texture_array_view(
        device: &ash::Device,
        image: vk::Image,
        format: vk::Format, // Обычно vk::Format::R8G8B8A8_SRGB
        layer_count: u32,   // Количество зарегистрированных текстур (блоков) в массиве
    ) -> vk::ImageView {
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            // КРИТИЧНО: Указываем видеокарте, что это именно МАССИВ картинок, а не одна текстура
            .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
            .format(format)
            // Настройка каналов (оставляем стандартное соответствие R->R, G->G и т.д.)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            // Описываем, какую часть текстуры мы открываем для шейдера
            .subresource_range(vk::ImageSubresourceRange {
                // Указываем, что это цветная текстура (не глубина)
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1, // Если будешь генерировать мип-мапы, здесь будет их количество
                base_array_layer: 0,
                layer_count, // Передаем, сколько всего слоев (блоков) у нас в массиве
            });

        unsafe {
            device
                .create_image_view(&view_info, None)
                .expect("Не удалось создать ImageView для массива текстур")
        }
    }

    pub fn create_instance(entry: Entry, instance_required_extensions: &[*const i8]) -> (Entry, ash::Instance) {
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

        let layer_names = [CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0").unwrap()];
        let layers_pointers: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let instance_info = InstanceCreateInfo::default()
            .enabled_extension_names(instance_required_extensions)
            // .enabled_layer_names(&layers_pointers)
            .application_info(&app_info);

        let instance = unsafe {
            entry
                .create_instance(&instance_info, None)
                .map_err(|e| format!("Error create insatnce with errror: {}", e))
                .unwrap()
        };

        (entry, instance)
    }
}

pub struct DepthBuffer {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub format: vk::Format,
}

impl DepthBuffer {
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        swapchain_extent: vk::Extent2D, // Разрешение вашего свопчейна (окна)
    ) -> Self {
        // 1. Выбираем формат
        let format = VkBuilder::find_depth_format(instance, physical_device);
        // println!("depth_buffer_format: {:?}", format);

        // 2. Описываем текстуру глубины
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: swapchain_extent.width,
                height: swapchain_extent.height,
                depth: 1, // Для экрана глубина всегда 1
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL) // Железо само упакует пиксели оптимально для видеокарты
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT) // Будет использоваться как буфер глубины
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = unsafe { device.create_image(&image_info, None).unwrap() };

        // 3. Выделяем память на GPU под эту текстуру
        let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

        // Используем вашу функцию find_memory_type, которую вы писали для обычных буферов
        let memory_type_index = find_memory_type(
            mem_properties,
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL, // Память должна быть строго внутри видеокарты
        )
        .expect("Не удалось найти память для текстуры глубины");

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { device.bind_image_memory(image, memory, 0).unwrap() };

        // 4. Создаем ImageView (интерфейс, через который RenderPass будет писать в текстуру)
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH, // Нас интересует только глубина (без стенсиля)
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let view = unsafe { device.create_image_view(&view_info, None).unwrap() };

        Self {
            image,
            memory,
            view,
            format,
        }
    }
}

pub struct SyncObjects {
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
}

pub fn create_command_pool_and_sync(
    device: &ash::Device,
    queue_family_index: u32,
    max_frames_in_flight: usize,
) -> (vk::CommandPool, SyncObjects) {
    // 1. Создаем пул команд
    let pool_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family_index)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER); // Чтобы переписывать буфер каждый кадр

    let command_pool = unsafe {
        device
            .create_command_pool(&pool_info, None)
            .expect("Не удалось создать CommandPool")
    };

    // 2. Создаем структуры синхронизации кадров
    let semaphore_info = vk::SemaphoreCreateInfo::default();
    let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED); // Важно: первый кадр не должен заблокировать CPU

    let mut image_available_semaphores = Vec::with_capacity(max_frames_in_flight);
    let mut render_finished_semaphores = Vec::with_capacity(max_frames_in_flight);
    let mut in_flight_fences = Vec::with_capacity(max_frames_in_flight);

    for _ in 0..max_frames_in_flight {
        unsafe {
            image_available_semaphores
                .push(device.create_semaphore(&semaphore_info, None).unwrap());
            render_finished_semaphores
                .push(device.create_semaphore(&semaphore_info, None).unwrap());
            in_flight_fences.push(device.create_fence(&fence_info, None).unwrap());
        }
    }

    let sync_objects = SyncObjects {
        image_available_semaphores,
        render_finished_semaphores,
        in_flight_fences,
    };

    (command_pool, sync_objects)
}

pub struct TextureArrayImage {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
}

impl TextureArrayImage {
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        resolution: u32,  // Размер одной текстуры блока (например, 16)
        layer_count: u32, // Количество зарегистрированных текстур-блоков
    ) -> Self {
        // 1. Описываем параметры 2D-массива текстур
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: resolution,
                height: resolution,
                depth: 1, // Для 2D-текстур глубина всегда 1
            })
            .mip_levels(1) // Если планируешь мип-маппинг, укажи количество уровней
            .array_layers(layer_count) // Самое важное: сколько слоев (текстур блоков) будет в массиве
            .format(vk::Format::R8G8B8A8_SRGB) // Стандартный формат: RGBA по 8 бит на канал, sRGB
            .tiling(vk::ImageTiling::OPTIMAL) // Видеокарта сама упакует пиксели в памяти «плитками» для максимальной скорости
            .initial_layout(vk::ImageLayout::UNDEFINED)
            // Текстура будет целью копирования (TRANSFER_DST) и будет читаться в шейдере (SAMPLED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .samples(vk::SampleCountFlags::TYPE_1) // Без мультисэмплинга
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = unsafe {
            device
                .create_image(&image_info, None)
                .expect("Не удалось создать VkImage для массива текстур")
        };

        // 2. Узнаем, сколько байт памяти этот массив текстур требует на конкретном GPU
        let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

        // 3. Ищем подходящий тип памяти на видеокарте (DEVICE_LOCAL — быстрая память VRAM)
        let mut memory_type_index = None;
        for i in 0..mem_properties.memory_type_count {
            if (mem_requirements.memory_type_bits & (1 << i)) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            {
                memory_type_index = Some(i);
                break;
            }
        }

        let memory_type_index = memory_type_index
            .expect("Не удалось найти подходящий тип DEVICE_LOCAL памяти для текстур");

        // 4. Выделяем физическую память на GPU
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe {
            device
                .allocate_memory(&alloc_info, None)
                .expect("Не удалось выделить память для массива текстур")
        };

        // 5. Привязываем выделенную память к нашему объекту текстуры
        unsafe {
            device
                .bind_image_memory(image, memory, 0)
                .expect("Не удалось связать память с VkImage");
        }

        Self { image, memory }
    }
}
