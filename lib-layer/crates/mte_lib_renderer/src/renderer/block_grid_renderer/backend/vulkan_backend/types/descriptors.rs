use ash::vk;
use ash::vk::*;

use crate::renderer::block_grid_renderer::backend::vulkan_backend::types::static_data::StaticData;

pub struct Descriptors {
    pub pool: vk::DescriptorPool,

    pub set0_layout: vk::DescriptorSetLayout,
    pub set1_layout: vk::DescriptorSetLayout,
    pub set2_layout: vk::DescriptorSetLayout,
    pub set3_layout: vk::DescriptorSetLayout,

    pub set0_blocks: vk::DescriptorSet,
    pub set1_camera: vk::DescriptorSet,
    pub set2_vectors: vk::DescriptorSet,
    pub set3_matrices: vk::DescriptorSet,
}

impl Descriptors {
    pub fn new(
        device: &ash::Device,
        resources: &StaticData,
        camera_buffer: vk::Buffer,
        vectors_buffer: vk::Buffer,
        matrices_buffer: vk::Buffer,
    ) -> Self {
        let layout_set0 = StaticData::get_set_layout(&device);

        // --- LAYOUT SET 1: Camera ---
        let binding_set1 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set1 = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set1)),
                    None,
                )
                .unwrap()
        };

        // Флаги привязок для Векторов (СТРОГО 1 элемент, так как биндинг один)
        let vector_binding_flags = [vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT];
        let mut vector_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&vector_binding_flags);

        // --- LAYOUT SET 2: Vectors ---
        let binding_set2 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1024) // Максимальный лимит страниц
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set2 = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set2))
                        .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                        .push_next(&mut vector_flags_info),
                    None,
                )
                .unwrap()
        };

        // Флаги привязок для Матриц (СТРОГО 1 элемент)
        let matrix_binding_flags = [vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT];
        let mut matrix_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&matrix_binding_flags);

        // --- LAYOUT SET 3: Matrices ---
        let binding_set3 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1024) // Максимальный лимит страниц
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set3 = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set3))
                        .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                        .push_next(&mut matrix_flags_info),
                    None,
                )
                .unwrap()
        };

        let layouts = [layout_set0, layout_set1, layout_set2, layout_set3];

        // 1. Создаем DescriptorPool с ПРАВИЛЬНЫМИ размерами
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1 + 1024 + 1024), // 1 (камера) + 1024 (вектора) + 1024 (матрицы)
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(4)
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND) // ОБЯЗАТЕЛЕН для UpdateAfterBind сетов
            .pool_sizes(&pool_sizes);

        let pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .unwrap()
        };

        // 2. Аллоцируем дескрипторные сеты с поддержкой VARIABLE_DESCRIPTOR_COUNT
        // Мы должны явно сказать Vulkan, сколько элементов мы ХОТИМ аллоцировать прямо сейчас.
        // Для сетов 0 и 1 передаем 0 (так как они фиксированные), для 2 и 3 — 1024.
        let variable_counts = [0, 0, 1024, 1024];
        let mut variable_count_alloc_info =
            vk::DescriptorSetVariableDescriptorCountAllocateInfo::default()
                .descriptor_counts(&variable_counts);

        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts)
            .push_next(&mut variable_count_alloc_info); // Указываем реальные каунты массивов

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap() };

        let set0_blocks = descriptor_sets[0];
        let set1_camera = descriptor_sets[1];
        let set2_vectors = descriptor_sets[2];
        let set3_matrices = descriptor_sets[3];

        // 3. Подготавливаем инфо-структуры для связывания стартовых ресурсов
        let texture_image_info = vk::DescriptorImageInfo::default()
            .image_view(resources.texture_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let sampler_image_info =
            vk::DescriptorImageInfo::default().sampler(resources.block_sampler);

        let block_props_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(resources.block_properties_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let camera_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(camera_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        // --- ВАЖНОЕ ИЗМЕНЕНИЕ ДЛЯ СТАРТА ---
        // Поскольку set2 и set3 теперь массивы, при первичной инициализации
        // мы привязываем твои стартовые базовые буферы (например, страницу №0)
        // строго в нулевой элемент массива.
        let vectors_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(vectors_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let matrices_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(matrices_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        // 4. Связываем ресурсы через update_descriptor_sets
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
            // --- SET 2 (Записываем стартовую страницу №0) ---
            vk::WriteDescriptorSet::default()
                .dst_set(set2_vectors)
                .dst_binding(0)
                .dst_array_element(0) // Индекс 0 в массиве дескрипторов
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&vectors_buffer_info)),
            // --- SET 3 (Записываем стартовую страницу №0) ---
            vk::WriteDescriptorSet::default()
                .dst_set(set3_matrices)
                .dst_binding(0)
                .dst_array_element(0) // Индекс 0 в массиве дескрипторов
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&matrices_buffer_info)),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };

        Self {
            pool,
            set0_layout: layout_set0,
            set1_layout: layout_set1,
            set2_layout: layout_set2,
            set3_layout: layout_set3,
            set0_blocks,
            set1_camera,
            set2_vectors,
            set3_matrices,
        }
    }

    pub fn chunks_sets(&self) -> [DescriptorSet; 3] {
        [self.set0_blocks, self.set1_camera, self.set2_vectors]
    }

    pub fn chunks_layouts(&self) -> [DescriptorSetLayout; 3] {
        [self.set0_layout, self.set1_layout, self.set2_layout]
    }

    pub fn phys_objects_sets(&self) -> [DescriptorSet; 4] {
        [
            self.set0_blocks,
            self.set1_camera,
            self.set2_vectors,
            self.set3_matrices,
        ]
    }

    pub fn phys_objects_layouts(&self) -> [DescriptorSetLayout; 4] {
        [
            self.set0_layout,
            self.set1_layout,
            self.set2_layout,
            self.set3_layout,
        ]
    }

    pub fn hand_sets(&self) -> [DescriptorSet; 1] {
        [self.set0_blocks]
    }

    pub fn hand_layouts(&self) -> [DescriptorSetLayout; 1] {
        [self.set0_layout]
    }

    pub fn update_matrix_pool(&self, device: &ash::Device, new_buffer: vk::Buffer, idx: u32) {
        self.update_pool(device, new_buffer, self.set3_matrices, idx);
    }

    pub fn update_vector_pool(&self, device: &ash::Device, new_buffer: vk::Buffer, idx: u32) {
        self.update_pool(device, new_buffer, self.set2_vectors, idx);
    }

    pub fn update_pool(
        &self,
        device: &ash::Device,
        new_buffer: vk::Buffer,
        set: DescriptorSet,
        idx: u32,
    ) {
        let new_buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(new_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let dyn_write = [vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .dst_array_element(idx)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&new_buffer_info))];

        unsafe { device.update_descriptor_sets(&dyn_write, &[]) };
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
        }
    }
}
