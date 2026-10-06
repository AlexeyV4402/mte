use std::ffi::CStr;
use std::sync::Arc;

use ash::vk::*;
use ash::{Entry, vk};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::{
    DepthBuffer, SyncObjects, VkBuilder, create_command_pool, create_sync
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::debug::init_debug_utils;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::framebuffer_object::FramebufferObject;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::swapchain_object::SwapchainObject;
pub struct VkBackend {
    pub entry: Entry,
    pub phys_dev: ash::vk::PhysicalDevice,
    pub instance: ash::Instance,
    pub memory_prop: PhysicalDeviceMemoryProperties,
    pub surface: ash::vk::SurfaceKHR,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,

    // Свопчейн и поверхности
    pub swapchain_object: SwapchainObject,

    pub depth_buffer: DepthBuffer,

    // Конвейер рендеринга
    pub render_pass: vk::RenderPass,
    pub framebuffer: FramebufferObject,

    pub cmd_pool: vk::CommandPool,
    pub sync_objects: SyncObjects,
    pub command_buffers: Vec<vk::CommandBuffer>,

    // Синхронизация кадров
    pub current_frame: usize,
    pub image_index: u32, // Индекс текущей картинки свопчейна, полученный в методе frame()
}

impl VkBackend {
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let entry = unsafe { Entry::load().unwrap() };

        let display_handle = window.display_handle().unwrap().as_raw();

        // 2. Запрашиваем у ash_window список расширений для ТЕКУЩЕЙ ОС и оконного сервера
        let raw_extensions = ash_window::enumerate_required_extensions(display_handle).unwrap();

        // 3. Переводим пойнтеры в массив для InstanceCreateInfo
        let mut instance_extensions: Vec<*const std::ffi::c_char> =
            raw_extensions.iter().map(|&ext| ext).collect();

        instance_extensions.push(ash::ext::debug_utils::NAME as *const CStr as *const i8);

        let (entry, instance) = VkBuilder::create_instance(entry, &instance_extensions);

        let surface: ash::vk::SurfaceKHR = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                display_handle.into(),
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
        // let phys_prop = unsafe { instance.get_physical_device_properties(phys_dev) };

        // if phys_prop.limits.max_push_constants_size < 160 {
        //     panic!("Видеоадаптер не соответствует требованиям");
        // }

        struct QueueFamilyInfo {
            queue_family_index: usize,
            _queue_prop: QueueFamilyProperties,
        }

        let mut queue_infos = vec![];

        for (index, i) in queue_family_prop.iter().enumerate() {
            queue_infos.push(QueueFamilyInfo {
                queue_family_index: index,
                _queue_prop: *i,
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

        let mut draw_parameters_features =
            vk::PhysicalDeviceShaderDrawParametersFeatures::default().shader_draw_parameters(true);

        let mut descriptor_indexing_features =
            vk::PhysicalDeviceDescriptorIndexingFeatures::default()
                // Разрешает флаг UPDATE_AFTER_BIND для Uniform-буферов (твоих пулов UBO)
                .descriptor_binding_uniform_buffer_update_after_bind(true)
                // Разрешает менять дескрипторы после записи в командный буфер (очень полезно)
                .descriptor_binding_partially_bound(true)
                // Разрешает массивы переменной длины (наши [] или [1024] в Bindless)
                .descriptor_binding_variable_descriptor_count(true);

        draw_parameters_features.p_next = &mut descriptor_indexing_features as *mut _ as *mut _;

        let features = PhysicalDeviceFeatures::default()
            .multi_draw_indirect(true)
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true);

        let device_info = DeviceCreateInfo::default()
            .enabled_features(&features)
            .queue_create_infos(&queue_family_infos)
            .enabled_extension_names(&phys_dev_required_extensions_ptrs)
            .push_next(&mut draw_parameters_features);

        let device = unsafe {
            instance
                .create_device(phys_dev, &device_info, None)
                .expect("Error create device")
        };

        init_debug_utils(&instance, &device);

        let main_graphics_queue = unsafe { device.get_device_queue(0, 0) };

        let surface_format = vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        };

        let swapchain_object = SwapchainObject::new(
            &entry,
            &instance,
            &device,
            phys_dev,
            surface,
            surface_format,
            window.inner_size(),
        );

        let image_count = swapchain_object.get_image_count();

        let (cmd_pool, sync_objects) = (
            create_command_pool(&device, 0),
            create_sync(&device, 2, image_count),
        );

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .level(vk::CommandBufferLevel::PRIMARY) // PRIMARY означает, что этот буфер можно отправить напрямую в видеокарту
            .command_buffer_count(2);

        let command_buffers = unsafe {
            device
                .allocate_command_buffers(&alloc_info)
                .expect("Не удалось аллоцировать Command Buffers")
        };

        let depth_buffer = DepthBuffer::new(
            &instance,
            &device,
            phys_dev,
            &memory_prop,
            swapchain_object.extent,
        );

        let render_pass =
            VkBuilder::create_render_pass(&device, surface_format, depth_buffer.format);

        let framebuffer =
            FramebufferObject::new(&device, &swapchain_object, &depth_buffer, render_pass);

        Self {
            entry,
            phys_dev,
            instance,
            memory_prop,
            surface,
            device,
            graphics_queue: main_graphics_queue,
            swapchain_object,
            depth_buffer,
            render_pass,
            framebuffer,
            cmd_pool,
            sync_objects,
            command_buffers,
            current_frame: 0,
            image_index: 0,
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
            self.framebuffer.destroy(&self.device);
            self.depth_buffer.destroy(&self.device);

            // ====================================================================
            // ШАГ 2: ПЕРЕСОЗДАНИЕ СВОПЧЕЙНА С НОВЫМ РАЗРЕШЕНИЕМ
            // ====================================================================
            let new_size = winit::dpi::PhysicalSize::new(new_width, new_height);

            // Задаем жесткий формат, который мы использовали при первой инициализации
            let surface_format = vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_SRGB,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };

            self.swapchain_object.recreate(
                &self.entry,
                &self.instance,
                &self.device,
                self.phys_dev,
                self.surface,
                surface_format,
                new_size,
            );

            let swapchain_image_count = self.swapchain_object.get_image_count();

            self.sync_objects
                .recreate_semaphores(swapchain_image_count, &self.device);

            // ====================================================================
            // ШАГ 3: СОЗДАНИЕ НОВОГО БУФЕРА ГЛУБИНЫ ПОД НОВЫЙ РАЗМЕР ОКНА
            // ====================================================================
            self.depth_buffer = DepthBuffer::new(
                &self.instance,
                &self.device,
                self.phys_dev,
                &self.memory_prop,
                self.swapchain_object.extent,
            );

            // ====================================================================
            // ШАГ 4: ПЕРЕСОЗДАНИЕ ФРЕЙМБУФЕРОВ
            // ====================================================================
            // Снова связываем новые ImageView свопчейна и новый общий буфер глубины
            self.framebuffer.recreate(
                &self.device,
                &self.swapchain_object,
                &self.depth_buffer,
                self.render_pass,
            );
        }
    }

    pub fn get_current_cmd(&self) -> vk::CommandBuffer {
        self.command_buffers[self.current_frame]
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
                .swapchain_object
                .loader
                .acquire_next_image(
                    self.swapchain_object.swapchain,
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

    pub fn end_frame<D: RenderData>(
        &mut self,
        data: &mut D,
    ) -> std::result::Result<(), ash::vk::Result> {
        let frame = self.current_frame;
        let img_idx = self.image_index as usize;
        let cmd = self.command_buffers[frame];

        unsafe {
            data.prepare_buffers(&self);

            // ====================================================================
            // ШАГ 2: НАЧАЛО RENDER PASS (ОЧИСТКА ЭКРАНА И ГЛУБИНЫ)
            // ====================================================================
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffer.framebuffers[self.image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_object.extent,
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
                .width(self.swapchain_object.extent.width as f32)
                .height(self.swapchain_object.extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0);

            let scissor = vk::Rect2D::default().extent(self.swapchain_object.extent);

            self.device.cmd_set_viewport(cmd, 0, &[viewport]);
            self.device.cmd_set_scissor(cmd, 0, &[scissor]);

            data.draw(&self);

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
            let present_swapchains = [self.swapchain_object.swapchain];
            let present_image_indices = [self.image_index];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&present_wait_semaphores)
                .swapchains(&present_swapchains)
                .image_indices(&present_image_indices);

            self.swapchain_object
                .loader
                .queue_present(self.graphics_queue, &present_info)
                .unwrap();
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

        self.current_frame = (self.current_frame + 1) % 2;

        std::result::Result::Ok(())
    }
}

pub trait RenderData {
    fn prepare_buffers(&mut self, renderer: &VkBackend);
    fn draw(&self, renderer: &VkBackend);
}
