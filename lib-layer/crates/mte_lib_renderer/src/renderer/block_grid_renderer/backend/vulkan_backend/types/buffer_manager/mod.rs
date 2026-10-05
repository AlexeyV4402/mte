use ash::vk;

pub mod render_buffer_manager;
pub mod slot_buffer_manager;

#[derive(PartialEq)]
pub struct BarrierData {
    pub dst_access_mask: vk::AccessFlags,
    pub buffer: vk::Buffer,
}
