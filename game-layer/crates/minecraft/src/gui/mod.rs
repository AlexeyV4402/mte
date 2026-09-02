use glam::Vec2;
use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::GuiIndexedPrimitive;
use lib_renderer::renderer::block_grid_renderer::render_objects::texture::TextureAtlas;
use lib_renderer::renderer::block_grid_renderer::types::vertex::GuiVertex;

pub struct GuiManager {
    static_gui: StaticGui,
    static_gui_atlas: TextureAtlas,
}

impl GuiManager {
    pub fn resize(&self, screen_size: Vec2, ui_scale: f32, renderer: Renderer) {
        let static_gui_primitive =
            self.static_gui
                .assemble(screen_size, ui_scale, &self.static_gui_atlas);
    }

    pub fn hide(&self) {}

    pub fn show(&self) {}
}

pub struct StaticGui {
    elements: Vec<GuiElement>,
}

pub struct GuiElement {
    pub anchor: Vec2,
    pub pivot: Vec2,
    pub offset_position: Vec2,
    pub size: Vec2,
}

impl GuiElement {
    pub fn calculate_screen_rect(&self, screen_size: Vec2, ui_scale: f32) -> (Vec2, Vec2) {
        let actual_size = self.size * ui_scale;
        let actual_offset = self.offset_position * ui_scale;

        let min = (screen_size * self.anchor) + actual_offset - (actual_size * self.pivot);

        let max = min + actual_size;

        (min, max)
    }
}

impl StaticGui {
    pub fn assemble(
        &self,
        screen_size: Vec2,
        ui_scale: f32,
        atlas: &TextureAtlas,
    ) -> GuiIndexedPrimitive {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        self.elements
            .iter()
            .enumerate()
            .for_each(|(start_idx, element)| {
                let (min, max) = element.calculate_screen_rect(screen_size, ui_scale);
                let top_left = min;
                let top_right = Vec2::new(max.x, min.y);
                let bottom_right = max;
                let bottom_left = Vec2::new(min.x, max.y);
                let (uv_min, uv_max) = atlas.get_uv_rect(top_left, bottom_right - top_left);
                // 1. Top-Left
                vertices.push(GuiVertex {
                    screen_pos: top_left.to_array(),
                    uv: uv_min.to_array(),
                    color: [1.0, 1.0, 1.0, 1.0],
                });

                // 2. Bottom-Left
                vertices.push(GuiVertex {
                    screen_pos: bottom_left.to_array(),
                    uv: Vec2::new(uv_min.x, uv_max.y).to_array(),
                    color: [1.0, 1.0, 1.0, 1.0],
                });

                // 3. Bottom-Right
                vertices.push(GuiVertex {
                    screen_pos: bottom_right.to_array(),
                    uv: uv_max.to_array(),
                    color: [1.0, 1.0, 1.0, 1.0],
                });

                // 4. Top-Right
                vertices.push(GuiVertex {
                    screen_pos: top_right.to_array(),
                    uv: Vec2::new(uv_max.x, uv_min.y).to_array(),
                    color: [1.0, 1.0, 1.0, 1.0],
                });

                indices.push(start_idx as u32);
                indices.push(start_idx as u32 + 1);
                indices.push(start_idx as u32 + 2);
                indices.push(start_idx as u32);
                indices.push(start_idx as u32 + 2);
                indices.push(start_idx as u32 + 3);
            });
        GuiIndexedPrimitive::new(vertices, indices)
    }
}
