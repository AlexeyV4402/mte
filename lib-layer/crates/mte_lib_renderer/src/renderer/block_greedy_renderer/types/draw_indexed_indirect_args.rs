use std::fmt;

use wgpu::wgt::DrawIndexedIndirectArgs;

// Создаешь обертку и вешаешь на нее derive(Clone, Copy)
#[derive(Clone, Copy)]
pub struct DebugDrawArgs(pub DrawIndexedIndirectArgs);

// Вручную реализуешь трейт Debug для своей обертки
impl fmt::Debug for DebugDrawArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawIndexedIndirectArgs")
            .field("index_count", &self.0.index_count)
            .field("instance_count", &self.0.instance_count)
            .field("first_index", &self.0.first_index)
            .field("base_vertex", &self.0.base_vertex)
            .field("base_instance", &self.0.first_instance)
            .finish()
    }
}