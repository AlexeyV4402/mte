use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;

pub struct MenuObject {}

impl MenuObject {
    pub fn update(&mut self, input_state: &InputState) {
        let mouse_pos = input_state.mouse_pos;
    }

    pub fn update_meshes(&mut self, renderer: &mut VkBackend) {}
}
