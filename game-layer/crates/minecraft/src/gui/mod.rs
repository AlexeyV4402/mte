// use glam::Vec2;
// use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::GuiIndexedPrimitive;
// use lib_renderer::renderer::block_grid_renderer::types::vertex::GuiVertex;

// pub struct StaticGui {
//     elements: Vec<GuiElement>,
// }

// pub struct GuiElement {
//     pub anchor: Vec2,
//     pub pivot: Vec2,
//     pub offset_position: Vec2,
//     pub size: Vec2,
//     pub uv_bl: Vec2,
//     pub uv_tr: Vec2,
// }

// impl GuiElement {
//     /// Главная функция: Вычисляет физические пиксельные координаты Квада (Min, Max)
//     /// на основе текущего размера окна и масштаба интерфейса.
//     pub fn calculate_screen_rect(&self, screen_size: Vec2, ui_scale: f32) -> (Vec2, Vec2) {
//         // 1. Применяем масштаб интерфейса к логическим размерам
//         let actual_size = self.size * ui_scale;
//         let actual_offset = self.offset_position * ui_scale;

//         // 2. Векторная магия: считаем позицию МИНИМУМА (верхний левый угол)
//         // Процессор за один такт сложит, умножит и вычтет X и Y одновременно
//         let min = (screen_size * self.anchor) + actual_offset - (actual_size * self.pivot);

//         // 3. Максимум — это просто смещение от минимума на размер элемента
//         let max = min + actual_size;

//         (min, max)
//     }
// }

// impl StaticGui {
//     pub fn assemble(&self, screen_size: Vec2, ui_scale: f32) -> GuiIndexedPrimitive {
//         let mut vertices = Vec::new();
//         let mut indices = Vec::new();
//         self.elements.iter().for_each(|element| {
//             let (min, max) = element.calculate_screen_rect(screen_size, ui_scale);
//             let top_left = min;
//             let top_right = Vec2::new(max.x, min.y);
//             let bottom_right = max;
//             let bottom_left = Vec2::new(min.x, max.y);
//             vertices.push(GuiVertex {
//                 screen_pos: top_left.to_array(),
//                 uv: ,
//                 color: todo!(),
//             });
//         });
//         GuiIndexedPrimitive::new(vertices, indices)
//     }
// }
