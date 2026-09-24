use std::ffi::CString;
use std::sync::OnceLock;

use ash::{Device, Entry, ext, vk};

pub static DEBUG_UTILS_DEVICE: OnceLock<ash::ext::debug_utils::Device> = OnceLock::new();

pub fn init_debug_utils(instance: &ash::Instance, device: &Device) {
    let loader = ext::debug_utils::Device::new(instance, device);
    let _ = DEBUG_UTILS_DEVICE.set(loader);
}

pub trait VulkanNameable {
    fn set_name(&self, name: &str);
}

impl VulkanNameable for vk::Buffer {
    fn set_name(&self, name: &str) {
        if let Some(loader) = DEBUG_UTILS_DEVICE.get() {
            if let Ok(c_name) = CString::new(name) {
                use ash::vk::Handle;

                let name_info = vk::DebugUtilsObjectNameInfoEXT {
                    s_type: vk::StructureType::DEBUG_UTILS_OBJECT_NAME_INFO_EXT,
                    p_next: std::ptr::null(),
                    object_type: vk::ObjectType::BUFFER,
                    object_handle: self.as_raw(),
                    p_object_name: c_name.as_ptr(),
                    _marker: std::marker::PhantomData,
                };

                unsafe {
                    let _ = loader.set_debug_utils_object_name(&name_info);
                }
            }
        }
    }
}
