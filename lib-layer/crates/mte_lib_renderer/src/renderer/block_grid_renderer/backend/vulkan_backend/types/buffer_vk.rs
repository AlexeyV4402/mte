use ash::vk;

pub struct VkBufferDataHV {
    pub buffer: vk::Buffer,
    pub mem: vk::DeviceMemory,
    pub mapped_ptr: *mut std::ffi::c_void,
}

impl VkBufferDataHV {
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&buffer_info, None).unwrap() };
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let memory_type_index = find_memory_type(
            mem_properties,
            mem_requirements.memory_type_bits,
            properties,
        )
        .expect("Не удалось найти подходящий тип памяти для буфера");

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0).unwrap() };

        let mapped_ptr = unsafe {
            device
                .map_memory(buffer_memory, 0, size, vk::MemoryMapFlags::empty())
                .expect("Не удалось замапить глобальный Staging")
        };

        Self {
            buffer,
            mem: buffer_memory,
            mapped_ptr,
        }
    }
}

pub struct VkBufferDataDL {
    pub buffer: vk::Buffer,
    mem: vk::DeviceMemory,
}

impl VkBufferDataDL {
    pub fn new(
        device: &ash::Device,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Self {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&buffer_info, None).unwrap() };
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let memory_type_index = find_memory_type(
            mem_properties,
            mem_requirements.memory_type_bits,
            properties,
        )
        .expect("Не удалось найти подходящий тип памяти для буфера");

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0).unwrap() };

        Self {
            buffer,
            mem: buffer_memory,
        }
    }

    pub fn destroy(self, device: &ash::Device) {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            device.free_memory(self.mem, None);
        }
    }
}

pub fn find_memory_type(
    mem_properties: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Option<u32> {
    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && mem_properties.memory_types[i as usize]
                .property_flags
                .contains(properties)
        {
            return Some(i);
        }
    }
    None
}
