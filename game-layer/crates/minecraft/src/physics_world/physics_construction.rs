use glam::{Mat3, Vec3};
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

pub struct ConstructUpdateResult {
    pub mass: f32,
    pub inverse_inertia: Mat3,
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

    pub fn update(&mut self) -> Option<ConstructUpdateResult> {
        let mut mass_sum = 0.0;
        let mut pos_sum: Vec3 = Vec3::ZERO;

        let mut box_max: Vec3 = Vec3::splat(f32::NEG_INFINITY);
        let mut box_min: Vec3 = Vec3::splat(f32::INFINITY);

        let mut has_solid_blocks = false;

        let mut raw_xx = 0.0;
        let mut raw_yy = 0.0;
        let mut raw_zz = 0.0;
        let mut raw_xy = 0.0;
        let mut raw_xz = 0.0;
        let mut raw_yz = 0.0;

        let block_mass = 1.0; // Масса одного вокселя
        let voxel_intrinsic_inertia = (1.0 / 6.0) * block_mass;

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

                raw_xx += block_mass * (center_pos.y * center_pos.y + center_pos.z * center_pos.z)
                    + voxel_intrinsic_inertia;
                raw_yy += block_mass * (center_pos.x * center_pos.x + center_pos.z * center_pos.z)
                    + voxel_intrinsic_inertia;
                raw_zz += block_mass * (center_pos.x * center_pos.x + center_pos.y * center_pos.y)
                    + voxel_intrinsic_inertia;

                raw_xy -= block_mass * center_pos.x * center_pos.y;
                raw_xz -= block_mass * center_pos.x * center_pos.z;
                raw_yz -= block_mass * center_pos.y * center_pos.z;
            }
        });

        // Если на корабле вообще нет блоков (все сломали), зануляем всё безопасными значениями
        if !has_solid_blocks {
            self.local_mass_center = Vec3::ZERO;
            self.mcr_box_max = Vec3::ZERO;
            self.mcr_box_min = Vec3::ZERO;
            return None;
        }

        // Честный центр масс
        self.local_mass_center = pos_sum / mass_sum;

        // Переносим границы коробки в пространство относительно центра масс
        self.mcr_box_max = box_max - self.local_mass_center;
        self.mcr_box_min = box_min - self.local_mass_center;

        let raw_inertia = Mat3::from_cols(
            Vec3::new(raw_xx, raw_xy, raw_xz),
            Vec3::new(raw_xy, raw_yy, raw_yz),
            Vec3::new(raw_xz, raw_yz, raw_zz),
        );

        // Смещаем тензор из (0,0,0) в новый центр масс (new_center).
        // По формуле мы вычитаем массу, умноженную на смещение центра.
        let cm_xx = mass_sum
            * (self.local_mass_center.y * self.local_mass_center.y
                + self.local_mass_center.z * self.local_mass_center.z);
        let cm_yy = mass_sum
            * (self.local_mass_center.x * self.local_mass_center.x
                + self.local_mass_center.z * self.local_mass_center.z);
        let cm_zz = mass_sum
            * (self.local_mass_center.x * self.local_mass_center.x
                + self.local_mass_center.y * self.local_mass_center.y);
        let cm_xy = -mass_sum * self.local_mass_center.x * self.local_mass_center.y;
        let cm_xz = -mass_sum * self.local_mass_center.x * self.local_mass_center.z;
        let cm_yz = -mass_sum * self.local_mass_center.y * self.local_mass_center.z;

        let center_shift_matrix = Mat3::from_cols(
            Vec3::new(cm_xx, cm_xy, cm_xz),
            Vec3::new(cm_xy, cm_yy, cm_yz),
            Vec3::new(cm_xz, cm_yz, cm_zz),
        );

        // Честный тензор инерции относительно центра масс — это разница этих матриц!
        let inertia_tensor = raw_inertia - center_shift_matrix;

        // Инвертируем и отдаем в физику
        let inverse_inertia = inertia_tensor.inverse();

        Some(ConstructUpdateResult {
            mass: mass_sum,
            inverse_inertia,
        })
    }
}
