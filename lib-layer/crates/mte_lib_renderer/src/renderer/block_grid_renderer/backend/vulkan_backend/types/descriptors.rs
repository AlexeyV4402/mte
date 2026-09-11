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
        matrices_buffer: vk::Buffer
    ) -> Self {
        // --- LAYOUT SET 0: StaticData ---
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

        // --- LAYOUT SET 2: Vectors ---
        let binding_set2 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set2 = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set2)),
                    None,
                )
                .unwrap()
        };

        // --- LAYOUT SET 3: Vectors ---
        let binding_set3 = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let layout_set3 = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default()
                        .bindings(std::slice::from_ref(&binding_set3)),
                    None,
                )
                .unwrap()
        };

        let layouts = [layout_set0, layout_set1, layout_set2, layout_set3];

        // 1. Создаем DescriptorPool под наши нужды
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1), // Block Textures
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLER)
                .descriptor_count(1), // Sampler
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1), // Block Properties / 
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(3), // WorldCameraUniform / Vectors / Matrices
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(4)
            .pool_sizes(&pool_sizes);

        let pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .unwrap()
        };

        // 2. Аллоцируем дескрипторные сеты
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap() };

        let set0_blocks = descriptor_sets[0];
        let set1_camera = descriptor_sets[1];
        let set2_vectors = descriptor_sets[2];
        let set3_matrices = descriptor_sets[3];

        // 3. Подготавливаем инфо-структуры для связывания ресурсов
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
            // --- SET 2 ---
            vk::WriteDescriptorSet::default()
                .dst_set(set2_vectors)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&vectors_buffer_info)),
            // --- SET 3 ---
            vk::WriteDescriptorSet::default()
                .dst_set(set3_matrices)
                .dst_binding(0)
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

    pub fn hand_sets(&self) -> [DescriptorSet; 2] {
        [self.set0_blocks, self.set3_matrices]
    }

    pub fn hand_layouts(&self) -> [DescriptorSetLayout; 2] {
        [self.set0_layout, self.set3_layout]
    }

    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            device.destroy_descriptor_pool(self.pool, None);
        }
    }
}
