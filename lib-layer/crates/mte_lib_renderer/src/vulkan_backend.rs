use std::sync::Arc;

use ash::Entry;
use ash::vk::*;

pub struct VkBackend {
    
}

impl VkBackend {
    pub fn new(window: Arc<winit::window::Window>) -> Self {
        let entry = unsafe { Entry::load().unwrap() };

        let version = unsafe { 
            entry.try_enumerate_instance_version()
        .expect("Error enumerate instance version") 
        };

        let api_version = match version {
            Some(version) => {
                version
            },
            None => {
                API_VERSION_1_0
            }
        };
        let app_info = ApplicationInfo::default()
            .application_name(c"Minecraft")
            .engine_name(c"MTE")
            .engine_version(0)
            .application_version(0)
            .api_version(api_version);
        let layers = unsafe { entry.enumerate_instance_layer_properties().expect("Error enumerate layers") };

        let extensions = unsafe {
            entry.enumerate_instance_extension_properties(None).expect("Error enumerate instance properties")
        };

        println!("------------Extensions------------");
        for i in &extensions {
            println!("{:?}", i.extension_name_as_c_str().unwrap_or(&c"None"));
        }

        todo!()
    }
}