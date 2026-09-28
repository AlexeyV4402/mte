use ash::vk::{self, ImageView, PhysicalDevice};
use winit::dpi::PhysicalSize;

pub struct SwapchainObject {
    pub swapchain: vk::SwapchainKHR,
    pub image_views: Vec<ImageView>,
    pub extent: vk::Extent2D,
    pub loader: ash::khr::swapchain::Device,
}

impl SwapchainObject {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        // capabilities: SurfaceCapabilitiesKHR,
        phys_dev: PhysicalDevice,
        surface: ash::vk::SurfaceKHR,
        surface_format: vk::SurfaceFormatKHR,
        window_size: PhysicalSize<u32>,
    ) -> Self {
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

        let loader = ash::khr::swapchain::Device::new(instance, device);

        let extent = if capabilities.current_extent.width != u32::MAX {
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
            .image_extent(extent)
            .image_array_layers(1) // Для обычного 2D экрана тут всегда 1 (больше нужно для VR/стерео)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // Картинка будет целью для рисования цвета
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE) // Монопольный доступ графической очереди
            .pre_transform(capabilities.current_transform) // Не переворачивать картинку (актуально для мобилок)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE) // Окно не прозрачное, обычный бэкграунд
            .present_mode(vk::PresentModeKHR::FIFO) // Жесткий V-Sync для детерминизма и лока FPS
            .clipped(true); // Разрешить Vulkan не рисовать пиксели, если окно закрыто другим окном ОС

        // // 7. Рожаем сам Swapchain на видеокарте
        let swapchain = unsafe {
            loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Пиздец, Swapchain не создался")
        };

        // // 8. Забираем хэндлы созданных картинок из VRAM
        let swapchain_images = unsafe { loader.get_swapchain_images(swapchain).unwrap() };

        let mut image_views = Vec::with_capacity(swapchain_images.len());

        // 2. В цикле проходим по каждой картинке из Swapchain
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
            image_views.push(image_view);
        }
        Self {
            swapchain,
            image_views,
            extent,
            loader,
        }
    }

    #[inline]
    pub fn get_image_count(&self) -> usize {
        unsafe {
            self.loader
                .get_swapchain_images(self.swapchain)
                .unwrap()
                .len()
        }
    }

    pub fn recreate(
        &mut self,
        entry: &ash::Entry,
        instance: &ash::Instance,
        device: &ash::Device,
        // capabilities: SurfaceCapabilitiesKHR,
        phys_dev: PhysicalDevice,
        surface: ash::vk::SurfaceKHR,
        surface_format: vk::SurfaceFormatKHR,
        window_size: PhysicalSize<u32>,
    ) {
        unsafe {
            for &view in &self.image_views {
                device.destroy_image_view(view, None);
            }
            self.image_views.clear();
            self.loader.destroy_swapchain(self.swapchain, None);
            let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
            let capabilities = surface_loader
                .get_physical_device_surface_capabilities(phys_dev, surface)
                .unwrap();

            // 3. Выбираем количество картинок в цепочке (обычно min + 1, чтобы был тройной буфер)
            let mut image_count = capabilities.min_image_count;
            if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
                image_count = capabilities.max_image_count;
            }

            let loader = ash::khr::swapchain::Device::new(instance, device);

            let extent = if capabilities.current_extent.width != u32::MAX {
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
                .image_extent(extent)
                .image_array_layers(1) // Для обычного 2D экрана тут всегда 1 (больше нужно для VR/стерео)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // Картинка будет целью для рисования цвета
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE) // Монопольный доступ графической очереди
                .pre_transform(capabilities.current_transform) // Не переворачивать картинку (актуально для мобилок)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE) // Окно не прозрачное, обычный бэкграунд
                .present_mode(vk::PresentModeKHR::FIFO) // Жесткий V-Sync для детерминизма и лока FPS
                .clipped(true); // Разрешить Vulkan не рисовать пиксели, если окно закрыто другим окном ОС

            // // 7. Рожаем сам Swapchain на видеокарте
            let swapchain = loader
                .create_swapchain(&swapchain_create_info, None)
                .expect("Пиздец, Swapchain не создался");

            // // 8. Забираем хэндлы созданных картинок из VRAM
            let swapchain_images = loader.get_swapchain_images(swapchain).unwrap();

            let mut image_views = Vec::with_capacity(swapchain_images.len());

            // 2. В цикле проходим по каждой картинке из Swapchain
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
                let image_view = device
                    .create_image_view(&view_create_info, None)
                    .expect("Не удалось создать VkImageView для кадра");

                // Сохраняем хэндл в наш массив
                image_views.push(image_view);
            }
            self.swapchain = swapchain;
            self.image_views = image_views;
            self.extent = extent;
        }
    }
}
