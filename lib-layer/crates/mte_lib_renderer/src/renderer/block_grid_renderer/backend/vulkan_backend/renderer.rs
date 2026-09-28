use std::ffi::CStr;
use std::sync::Arc;

use ash::vk::*;
use ash::{Entry, vk};
use glam::Mat4;
use mte_macros::vfs_include_vk_shader;
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};

use crate::renderer::block_grid_renderer::backend::vulkan_backend::builder::{
    DepthBuffer, SyncObjects, VkBuilder, create_command_pool, create_sync
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::debug::init_debug_utils;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::indirect_buffer_manager::{
    ChunkGpuHandle, HandGpuHandle, IndirectBufferManager, PhysObjectGpuHandle
};
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::buffer_vk::VkBufferDataHV;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::descriptors::Descriptors;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::framebuffer_object::FramebufferObject;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::static_data::StaticData;
use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::swapchain_object::SwapchainObject;
use crate::renderer::block_grid_renderer::render_objects::camera::WorldCameraUniform;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use crate::renderer::block_grid_renderer::types::RendererCreateArgs;

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

    pub chunks_pipeline_layout: vk::PipelineLayout,
    pub chunks_pipeline: vk::Pipeline,

    pub hand_pipeline_layout: vk::PipelineLayout,
    pub hand_pipeline: vk::Pipeline,

    pub phys_objects_pipeline_layout: vk::PipelineLayout,
    pub phys_objects_pipeline: vk::Pipeline,

    pub cmd_pool: vk::CommandPool,
    pub sync_objects: SyncObjects,
    pub command_buffers: Vec<vk::CommandBuffer>,

    // Синхронизация кадров
    pub current_frame: usize,
    pub image_index: u32, // Индекс текущей картинки свопчейна, полученный в методе frame()

    // --- НАШ МЕНЕДЖЕР БУФЕРОВ ---
    pub buffer_manager: IndirectBufferManager,

    // Буфер для камеры на GPU (HOST_VISIBLE | HOST_COHERENT) и замаппленный указатель на него
    pub camera_buffer: VkBufferDataHV,

    pub descriptors: Descriptors,

    pub static_data: StaticData,

    pub hand_mat: [[f32; 4]; 4],
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
        let phys_objects_vert = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/phys_objects.vertex.glsl"
        );
        let phys_objects_frag = vfs_include_vk_shader!(
            "workspace://game-layer/assets/minecraft/shaders/phys_objects.fragment.glsl"
        );

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

        let chunks_vert_shader_module = VkBuilder::create_shader_module(&device, chunks_vert);
        let chunks_frag_shader_module = VkBuilder::create_shader_module(&device, chunks_frag);
        let hand_vert_shader_module = VkBuilder::create_shader_module(&device, hand_vert);
        let hand_frag_shader_module = VkBuilder::create_shader_module(&device, hand_frag);
        let phys_objects_vert_shader_module =
            VkBuilder::create_shader_module(&device, phys_objects_vert);
        let phys_objects_frag_shader_module =
            VkBuilder::create_shader_module(&device, phys_objects_frag);

        let mut buffer_manager = IndirectBufferManager::new(&device, &memory_prop);

        let camera_buffer = VkBufferDataHV::new(
            &device,
            &memory_prop,
            size_of::<WorldCameraUniform>() as u64,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::UNIFORM_BUFFER,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );

        let static_data = StaticData::new(
            &device,
            &memory_prop,
            renderer_create_args.block_properties.len(),
            renderer_create_args.layer_count,
        );

        let descriptors = Descriptors::new(
            &device,
            &static_data,
            camera_buffer.buffer,
            buffer_manager.vector_buffer.gpu_buffers[0].buffer,
            buffer_manager.matrix_buffer.gpu_buffers[0].buffer,
        );

        let chunks_pipeline_layout =
            VkBuilder::create_pipeline_layout(&device, &descriptors.chunks_layouts());

        let chunks_pipeline = VkBuilder::create_graphics_pipeline(
            &device,
            chunks_pipeline_layout,
            render_pass,
            chunks_vert_shader_module,
            chunks_frag_shader_module,
        );

        let phys_objects_pipeline_layout =
            VkBuilder::create_pipeline_layout(&device, &descriptors.phys_objects_layouts());

        let phys_objects_pipeline = VkBuilder::create_graphics_pipeline(
            &device,
            phys_objects_pipeline_layout,
            render_pass,
            phys_objects_vert_shader_module,
            phys_objects_frag_shader_module,
        );

        let hand_pipeline_layout = VkBuilder::create_pipeline_layout_with_push_const_range(
            &device,
            &descriptors.hand_layouts(),
            64,
        );

        let hand_pipeline = VkBuilder::create_graphics_pipeline(
            &device,
            hand_pipeline_layout,
            render_pass,
            hand_vert_shader_module,
            hand_frag_shader_module,
        );

        unsafe {
            StaticData::upload(
                &device,
                main_graphics_queue,
                cmd_pool,
                &mut buffer_manager,
                static_data.block_properties_buffer.buffer,
                static_data.texture_array.image,
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
            swapchain_object,
            depth_buffer,
            render_pass,
            framebuffer,
            chunks_pipeline_layout,
            chunks_pipeline,
            hand_pipeline_layout,
            hand_pipeline,
            phys_objects_pipeline_layout,
            phys_objects_pipeline,
            cmd_pool,
            sync_objects,
            command_buffers,
            current_frame: 0,
            image_index: 0,
            buffer_manager,
            camera_buffer,
            descriptors,
            static_data,
            hand_mat: Mat4::IDENTITY.to_cols_array_2d(),
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
            current_cmd,
        );

        handle
    }

    pub fn load_phys_object(
        &mut self,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
        matrix: [[f32; 4]; 4],
    ) -> Option<PhysObjectGpuHandle> {
        let current_cmd = self.command_buffers[self.current_frame];

        let handle = self.buffer_manager.load_phys_object(
            primitive,
            vector,
            matrix,
            &self.device,
            &self.memory_prop,
            current_cmd,
        );

        handle
    }

    pub fn update_phys_object(
        &mut self,
        old_handle: PhysObjectGpuHandle,
        vector: [i32; 4],
        matrix: [[f32; 4]; 4],
    ) -> PhysObjectGpuHandle {
        let current_cmd = self.command_buffers[self.current_frame];

        let handle = self.buffer_manager.update_phys_object(
            old_handle,
            vector,
            matrix,
            &self.device,
            &self.memory_prop,
            current_cmd,
        );

        handle
    }

    pub fn unload_phys_object(&mut self, handle: PhysObjectGpuHandle) {
        self.buffer_manager.unload_phys_object(handle);
    }

    pub fn load_hand(
        &mut self,
        primitive: BlockIndexedPrimitive,
        matrix: [[f32; 4]; 4],
    ) -> Option<HandGpuHandle> {
        self.hand_mat = matrix;
        let current_cmd = self.command_buffers[self.current_frame];

        let handle =
            self.buffer_manager
                .load_hand(primitive, &self.device, &self.memory_prop, current_cmd);

        handle
    }

    pub fn unload_chunk(&mut self, handle: ChunkGpuHandle) {
        self.buffer_manager.unload_chunk(handle);
    }

    pub fn unload_hand(&mut self, data: HandGpuHandle) {
        self.hand_mat = Mat4::IDENTITY.to_cols_array_2d();
        self.buffer_manager.unload_hand(data);
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
                .buffer(self.buffer_manager.vector_buffer.gpu_buffers[0].buffer) // Достаем VkBuffer твоего PageBuffer
                .offset(0)
                .size(vk::WHOLE_SIZE);

            let matrix_barrier = vk::BufferMemoryBarrier::default()
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
                matrix_barrier,
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

            // ====================================================================
            // ШАГ 4: ВКЛЮЧАЕМ КОНВЕЙЕР И ДЕСКРИПТОРЫ
            // ====================================================================

            self.device.cmd_bind_vertex_buffers(
                cmd,
                0,
                &[self.buffer_manager.vertex_buffer.gpu_buffers[0].buffer],
                &[0],
            );

            self.device.cmd_bind_index_buffer(
                cmd,
                self.buffer_manager.index_buffer.gpu_buffers[0].buffer,
                0,
                vk::IndexType::UINT32,
            );

            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.chunks_pipeline,
            );

            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.chunks_pipeline_layout,
                0,
                &self.descriptors.chunks_sets(),
                &[],
            );

            // ====================================================================
            // ШАГ 5: ЖЕЛЕЗНЫЙ МУЛЬТИ-ДРОУ ВЫЗОВ (ОТРИСОВКА МИРА)
            // ====================================================================

            let stride = std::mem::size_of::<vk::DrawIndexedIndirectCommand>() as u32;
            let chunks_slots = IndirectBufferManager::ONE_VECTOR_BUFFER_SLOTS_COUNT as u32;

            self.device.cmd_draw_indexed_indirect(
                cmd,
                self.buffer_manager
                    .indirect_buffer
                    .gpu_indexed_indirect_buffer
                    .buffer,
                stride as vk::DeviceSize,
                chunks_slots,
                stride,
            );

            self.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.phys_objects_pipeline,
            );

            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.phys_objects_pipeline_layout,
                0,
                &self.descriptors.phys_objects_sets(),
                &[],
            );

            let phys_objects_slots = IndirectBufferManager::ONE_MATRIX_BUFFER_SLOTS_COUNT as u32;

            self.device.cmd_draw_indexed_indirect(
                cmd,
                self.buffer_manager
                    .indirect_buffer
                    .gpu_indexed_indirect_buffer
                    .buffer,
                stride as vk::DeviceSize * (1 + chunks_slots as u64),
                phys_objects_slots,
                stride,
            );

            let clear_attachment = vk::ClearAttachment::default()
                .aspect_mask(vk::ImageAspectFlags::DEPTH) // Стираем только карту глубины кадра
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                });

            let clear_rect = vk::ClearRect::default()
                .rect(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_object.extent,
                })
                .layer_count(1);

            self.device
                .cmd_clear_attachments(cmd, &[clear_attachment], &[clear_rect]);

            // Включаем конвейер руки
            self.device
                .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.hand_pipeline);

            self.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.hand_pipeline_layout,
                0,
                &self.descriptors.hand_sets(),
                &[],
            );

            // ТОЛКАЕМ МАТРИЦУ РУКИ НА GPU (Push-константы)
            let hand_bytes = bytemuck::cast_slice(&self.hand_mat);
            self.device.cmd_push_constants(
                cmd,
                self.hand_pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                &hand_bytes,
            );

            // Вызываем INDIRECT-отрисовку руки из САМОГО НАЧАЛА буфера (Слот №0)
            self.device.cmd_draw_indexed_indirect(
                cmd,
                self.buffer_manager
                    .indirect_buffer
                    .gpu_indexed_indirect_buffer
                    .buffer,
                0, // Смещение 0 байт — читаем строго Слот №0!
                1, // Рисуем строго одну команду (нашу руку)
                stride,
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

    pub fn update_camera(&mut self, pass_1_camera_uniform: WorldCameraUniform) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                &pass_1_camera_uniform as *const WorldCameraUniform,
                self.camera_buffer.mapped_ptr as *mut WorldCameraUniform,
                1,
            );
        }
    }
}
