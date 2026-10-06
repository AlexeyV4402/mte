use std::mem;

use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::game_data::VkInGameData;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::types::game_buffer_manager::PhysObjectGpuHandle;

use crate::physics_world::components::Components;

#[derive(Default)]
pub struct RenderModule {
    pub object_handles: Vec<Option<PhysObjectGpuHandle>>,

    pub show_queue: Vec<usize>,
    pub hide_queue: Vec<usize>,
    pub update_queue: Vec<usize>,
}

impl RenderModule {
    pub fn update_meshes(
        &mut self,
        renderer_data: &mut VkInGameData,
        renderer: &VkBackend,
        components: &Components,
    ) {
        if self.show_queue.len() > 0 {
            let mut queue = mem::replace(&mut self.show_queue, Vec::with_capacity(32));
            queue.drain(..).for_each(|idx| {
                let mesh = components.construct_data[idx].get_mesh();
                if self.object_handles.len() > idx {
                    if let Some(old_handle) = self.object_handles[idx] {
                        renderer_data.unload_phys_object(old_handle);
                    }
                } else {
                    self.object_handles.push(None);
                }

                self.object_handles[idx] = renderer_data.load_phys_object(
                    renderer,
                    mesh,
                    components.get_vector(idx),
                    components.get_matrix(idx),
                );
            });
        }
        let mut queue = mem::replace(&mut self.hide_queue, Vec::with_capacity(32));
        queue.drain(..).for_each(|idx| {
            if let Some(handle) = self.object_handles[idx] {
                renderer_data.unload_phys_object(handle);
                self.object_handles[idx] = None;
            }
        });
        let mut queue = mem::replace(&mut self.update_queue, Vec::with_capacity(32));
        queue.drain(..).for_each(|idx| {
            if let Some(handle) = self.object_handles[idx] {
                renderer_data.update_phys_object(
                    renderer,
                    handle,
                    components.get_vector(idx),
                    components.get_matrix(idx),
                );
            }
        });
    }
}
