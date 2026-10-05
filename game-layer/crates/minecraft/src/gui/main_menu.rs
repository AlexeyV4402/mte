use std::ops::Div;

use glam::Vec2;
use lib_io::user_io::InputState;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::gui_renderer::VkGuiBackend;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::game_renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::GuiIndexedPrimitive;
use lib_renderer::renderer::block_grid_renderer::types::vertex::GuiVertex;

enum GuiEvent {

}

pub struct MainMenu {
    elements: [GuiElement; 3],
    event_queue: Vec<GuiEvent>
}

impl MainMenu {
    pub const fn new() -> Self {
        Self {
            elements: [
                GuiElement::new(
                    Vec2::new(-0.75, -0.75),
                    Vec2::new(0.75, -0.5),
                    [0.5, 0.5, 0.5, 0.5],
                    [0.75, 0.75, 0.75, 0.75],
                    [0.25, 0.25, 0.25, 0.25],
                    VidgetType::Button,
                    VidgetState::None,
                ),
                GuiElement::new(
                    Vec2::new(-0.75, -0.25),
                    Vec2::new(0.75, 0.0),
                    [0.5, 0.5, 0.5, 0.5],
                    [0.75, 0.75, 0.75, 0.75],
                    [0.25, 0.25, 0.25, 0.25],
                    VidgetType::Button,
                    VidgetState::None,
                ),
                GuiElement::new(
                    Vec2::new(-0.75, 0.25),
                    Vec2::new(0.75, 0.5),
                    [0.5, 0.5, 0.5, 0.5],
                    [0.75, 0.75, 0.75, 0.75],
                    [0.25, 0.25, 0.25, 0.25],
                    VidgetType::Button,
                    VidgetState::None,
                ),
            ],
            event_queue: Vec::new(),
            
        }
    }

    pub fn update_meshes(&self, renderer: &mut VkGuiBackend) {
        self.elements.iter().enumerate().for_each(|(idx, element)| {
            renderer.load_quad(element.get_mesh(), idx as u64);
        });
    }

    pub fn update(&mut self, input_state: &InputState, screen_size: Vec2) {
        let normalized_mouse = input_state.mouse_pos.div(screen_size);

        // 2. Переводим в NDC Vulkan: [0.0; 1.0] -> [-1.0; 1.0]
        let mouse_pos = Vec2::new(
            normalized_mouse.x * 2.0 - 1.0,
            normalized_mouse.y * 2.0 - 1.0,
        );
        self.elements
            .iter_mut()
            .for_each(|element| match element.vidget_type {
                VidgetType::None => todo!(),
                VidgetType::Button => {
                    if mouse_pos.x < element.pos_max.x
                        && mouse_pos.y < element.pos_max.y
                        && mouse_pos.x > element.pos_min.x
                        && mouse_pos.y > element.pos_min.y
                    {
                        if input_state.is_mouse_down(winit::event::MouseButton::Left) {
                            element.vidget_state = VidgetState::Down
                        } else {
                            element.vidget_state = VidgetState::Hover
                        }
                    } else {
                        element.vidget_state = VidgetState::None
                    }
                }
            });
    }
}

pub enum VidgetType {
    None,
    Button,
}

pub enum VidgetState {
    None,
    Hover,
    Down,
}

pub struct GuiElement {
    pos_min: Vec2,
    pos_max: Vec2,
    default_color: [f32; 4],
    hover_color: [f32; 4],
    down_color: [f32; 4],
    vidget_type: VidgetType,
    vidget_state: VidgetState,
}

impl GuiElement {
    #[inline]
    pub const fn new(
        pos_min: Vec2,
        pos_max: Vec2,
        default_color: [f32; 4],
        hover_color: [f32; 4],
        down_color: [f32; 4],
        vidget_type: VidgetType,
        vidget_state: VidgetState,
    ) -> Self {
        Self {
            pos_min,
            pos_max,
            default_color,
            hover_color,
            down_color,
            vidget_type,
            vidget_state,
        }
    }

    pub fn fill_mesh(&self, vertices: &mut Vec<GuiVertex>, indices: &mut Vec<u32>) {
        let start_idx = vertices.len();
        let top_left = self.pos_min;
        let top_right = Vec2::new(self.pos_max.x, self.pos_min.y);
        let bottom_right = self.pos_max;
        let bottom_left = Vec2::new(self.pos_min.x, self.pos_max.y);

        let color = match self.vidget_state {
            VidgetState::None => self.default_color,
            VidgetState::Hover => self.hover_color,
            VidgetState::Down => self.down_color,
        };

        // 1. Top-Left
        vertices.push(GuiVertex {
            screen_pos: top_left.to_array(),
            color,
        });

        // 2. Bottom-Left
        vertices.push(GuiVertex {
            screen_pos: bottom_left.to_array(),
            color,
        });

        // 3. Bottom-Right
        vertices.push(GuiVertex {
            screen_pos: bottom_right.to_array(),
            color,
        });

        // 4. Top-Right
        vertices.push(GuiVertex {
            screen_pos: top_right.to_array(),
            color,
        });

        indices.push(start_idx as u32);
        indices.push(start_idx as u32 + 1);
        indices.push(start_idx as u32 + 2);
        indices.push(start_idx as u32);
        indices.push(start_idx as u32 + 2);
        indices.push(start_idx as u32 + 3);
    }

    pub fn get_mesh(&self) -> GuiIndexedPrimitive {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let start_idx = 0;
        let top_left = self.pos_min;
        let top_right = Vec2::new(self.pos_max.x, self.pos_min.y);
        let bottom_right = self.pos_max;
        let bottom_left = Vec2::new(self.pos_min.x, self.pos_max.y);

        let color = match self.vidget_state {
            VidgetState::None => self.default_color,
            VidgetState::Hover => self.hover_color,
            VidgetState::Down => self.down_color,
        };

        // 1. Top-Left
        vertices.push(GuiVertex {
            screen_pos: top_left.to_array(),
            color,
        });

        // 2. Bottom-Left
        vertices.push(GuiVertex {
            screen_pos: bottom_left.to_array(),
            color,
        });

        // 3. Bottom-Right
        vertices.push(GuiVertex {
            screen_pos: bottom_right.to_array(),
            color,
        });

        // 4. Top-Right
        vertices.push(GuiVertex {
            screen_pos: top_right.to_array(),
            color,
        });

        indices.push(start_idx as u32);
        indices.push(start_idx as u32 + 1);
        indices.push(start_idx as u32 + 2);
        indices.push(start_idx as u32);
        indices.push(start_idx as u32 + 2);
        indices.push(start_idx as u32 + 3);
        GuiIndexedPrimitive::new(vertices, indices)
    }
}
