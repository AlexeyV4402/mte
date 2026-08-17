use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use lib_renderer::renderer::block_grid_renderer::types::BlockVertex;

use crate::types::blocks::block::Block;
use crate::utils::mesher::{SIDE_BOTTOM, SIDE_EAST, SIDE_NORTH, SIDE_SOUTH, SIDE_TOP, SIDE_WEST};

#[derive(Clone, Copy)]
pub struct Item {
    pub item_type: ItemType,
    pub count: u32,
}

#[derive(Clone, Copy)]
pub enum ItemType {
    Block(Block),
    Item(TrueItem),
}

impl Item {
    pub fn from_block(block: Block, count: u32) -> Self {
        Self {
            item_type: ItemType::Block(block),
            count,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TrueItem {}

impl ItemType {
    pub fn get_hand_model(self) -> BlockIndexedPrimitive {
        match self {
            ItemType::Block(block) => {
                let material_id = block.get_type() as u32;
                // println!("Материал: {}", material_id);
                let mut vertices = Vec::with_capacity(24);
                let mut indices = Vec::with_capacity(36);

                // Вспомогательное замыкание для быстрой сборки квада (грани куба)
                // Принимает 4 локальные вершины, ID стороны и ID материала
                let mut add_face = |p0: (u32, u32, u32),
                                    p1: (u32, u32, u32),
                                    p2: (u32, u32, u32),
                                    p3: (u32, u32, u32),
                                    side_id: u32| {
                    let start_idx = vertices.len() as u32;

                    // Добавляем 4 вершины для квада
                    vertices.push(BlockVertex::new(p0.0, p0.1, p0.2, side_id, material_id));
                    vertices.push(BlockVertex::new(p1.0, p1.1, p1.2, side_id, material_id));
                    vertices.push(BlockVertex::new(p2.0, p2.1, p2.2, side_id, material_id));
                    vertices.push(BlockVertex::new(p3.0, p3.1, p3.2, side_id, material_id));

                    // Строим два треугольника для этой грани (стандартный порядок обхода)
                    indices.extend_from_slice(&[
                        start_idx,
                        start_idx + 1,
                        start_idx + 2,
                        start_idx,
                        start_idx + 2,
                        start_idx + 3,
                    ]);
                };

                // TOP (Верхняя грань, Y = 1)
                add_face((0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0), SIDE_TOP);

                // BOTTOM (Нижняя грань, Y = 0)
                add_face((0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1), SIDE_BOTTOM);

                // NORTH (Северная грань, Z = 0)
                add_face((1, 0, 0), (0, 0, 0), (0, 1, 0), (1, 1, 0), SIDE_NORTH);

                // SOUTH (Южная грань, Z = 1)
                add_face((0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1), SIDE_SOUTH);

                // WEST (Западная грань, X = 0)
                add_face((0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0), SIDE_WEST);

                // EAST (Восточная грань, X = 1)
                add_face((1, 0, 1), (1, 0, 0), (1, 1, 0), (1, 1, 1), SIDE_EAST);

                BlockIndexedPrimitive::new(vertices, indices)
            }
            ItemType::Item(true_item) => todo!(),
        }
    }
}
