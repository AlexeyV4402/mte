use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::gui_data::VkGuiData;

use crate::gui::main_menu::MainMenu;

pub struct MainMenuState {
    pub vk_data: VkGuiData,
    pub main_menu: MainMenu,
}

impl MainMenuState {
    pub fn new(device: &ash::Device, mem_properties: &ash::vk::PhysicalDeviceMemoryProperties, render_pass: ash::vk::RenderPass) -> Self {
        Self {
            vk_data: VkGuiData::new(device, mem_properties, render_pass),
            main_menu: MainMenu::new(),
        }
    }

    pub fn destroy(self, device: &ash::Device) {
        self.vk_data.destroy(device);
        drop(self.main_menu);
    }
}