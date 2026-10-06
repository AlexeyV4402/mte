use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::game_data::VkInGameData;

use crate::types::world::World;
use crate::utils::world_generator::SuperSimplexGenerator;

pub struct InGameState {
    pub vk_data: VkInGameData,
    pub world: World<SuperSimplexGenerator>,
}
