use glam::Vec3;
use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;

use crate::types::blocks::block::Block;
use crate::types::coordinates::core::MiniChunkCoords;
use crate::utils::mesher::{PhysMiniChunk8, generate_mesh_generic, into_prerender_array};

pub struct PhysicsConstruction {
    pub blocks: [Block; 512],
    pub local_mass_center: Vec3,
    pub mcr_box_min: Vec3, // Mass Center Related
    pub mcr_box_max: Vec3,
}

impl PhysicsConstruction {
    pub fn one_block(block: Block) -> Self {
        let mut blocks = [Block::air(); 512];
        let idx = usize::from(MiniChunkCoords::new(4, 4, 4));
        blocks[idx] = block;
        PhysicsConstruction {
            blocks,
            local_mass_center: Vec3::new(4.5, 4.5, 4.5),
            mcr_box_min: Vec3::new(-0.5, -0.5, -0.5),
            mcr_box_max: Vec3::new(0.5, 0.5, 0.5),
        }
    }

    pub fn get_block(&self, coords: MiniChunkCoords) -> Block {
        self.blocks[usize::from(coords)]
    }

    pub fn get_mesh(&self) -> BlockIndexedPrimitive {
        let arr = into_prerender_array(&self.blocks);

        generate_mesh_generic::<PhysMiniChunk8, u8>(&arr, 0xFF)
    }

    pub fn update_box(&mut self) -> bool {
        let mut mass_sum = 0.0;
        let mut pos_sum: Vec3 = Vec3::ZERO;

        // Инициализируем min бесконечностью, а max - минус бесконечностью
        let mut box_max: Vec3 = Vec3::splat(f32::NEG_INFINITY);
        let mut box_min: Vec3 = Vec3::splat(f32::INFINITY);

        let mut has_solid_blocks = false;

        self.blocks.iter().enumerate().for_each(|(idx, block)| {
            // Обязательно проверяем, что это не воздух!
            if block.get_type().is_solid() {
                has_solid_blocks = true;

                // Масса одного блока (допустим, 1.0)
                mass_sum += 1.0;

                // Получаем чистую левую-нижнюю координату вокселя (например, от 0.0 до 7.0)
                let block_origin = Vec3::from(MiniChunkCoords::from(idx).0.as_f32());

                // Центр вокселя для расчета центра масс
                let center_pos = block_origin + Vec3::splat(0.5);
                pos_sum += center_pos;

                // Для коробки min - это левый угол блока, а max - правый дальний (+ 1.0)
                box_min = box_min.min(block_origin);
                box_max = box_max.max(block_origin + Vec3::ONE);
            }
        });

        // Если на корабле вообще нет блоков (все сломали), зануляем всё безопасными значениями
        if !has_solid_blocks {
            self.local_mass_center = Vec3::ZERO;
            self.mcr_box_max = Vec3::ZERO;
            self.mcr_box_min = Vec3::ZERO;
            return false;
        }

        // Честный центр масс
        self.local_mass_center = pos_sum / mass_sum;

        // Переносим границы коробки в пространство относительно центра масс
        self.mcr_box_max = box_max - self.local_mass_center;
        self.mcr_box_min = box_min - self.local_mass_center;
        true
    }
}
