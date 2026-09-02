use std::mem::type_info::Generic;
use std::time::Instant;

use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use rustc_hash::FxHashMap;

use crate::types::blocks::block::{Block, BlockType};
use crate::types::chunk::Chunk;
use crate::types::coordinates::core::{ChunkCoords, GlobalCoords, InternalCoords};
use crate::world_generator::WorldGenerator;

pub struct Dimension {
    chunks: FxHashMap<ChunkCoords, Chunk>,
    dirty_chunks: Vec<ChunkCoords>,
}

impl Dimension {
    pub fn new() -> Self {
        // let start = Instant::now();
        let chunks: FxHashMap<ChunkCoords, Chunk> = Default::default();
        let dirty_chunks = Vec::new();
        // for x in 0..=7 {
        //     for y in 0..=0 {
        //         for z in 0..=7 {
        //             chunks.insert(
        //                 (x, y, z).into(),
        //                 // Chunk::from_block_as_grid(Block::from_type(BlockType::Dirt), 2),
        //                 Chunk::from_block(Block::from_type(BlockType::Dirt)),
        //             );
        //             dirty_chunks.push((x, y, z).into());
        //         }
        //     }
        // }
        // println!(
        //     "Генерация {} чанков: {} ms",
        //     chunks.len(),
        //     start.elapsed().as_millis()
        // );
        Self {
            chunks,
            dirty_chunks,
        }
    }

    pub fn generate_chunk<Generator>(&mut self, chunk_coords: ChunkCoords, generator: &Generator) 
    where Generator: WorldGenerator 
    {
        self.chunks.insert(chunk_coords, generator.generate_chunk(chunk_coords));
        self.dirt_chunk(chunk_coords);
    }

    pub fn new() -> Self {
        let start = Instant::now();
        let mut chunks: FxHashMap<ChunkCoords, Chunk> = Default::default();
        let mut dirty_chunks = Vec::new();
        for x in 0..=7 {
            for y in 0..=0 {
                for z in 0..=7 {
                    chunks.insert(
                        (x, y, z).into(),
                        // Chunk::from_block_as_grid(Block::from_type(BlockType::Dirt), 2),
                        Chunk::from_block(Block::from_type(BlockType::Dirt)),
                    );
                    dirty_chunks.push((x, y, z).into());
                }
            }
        }
        println!(
            "Генерация {} чанков: {} ms",
            chunks.len(),
            start.elapsed().as_millis()
        );
        Self {
            chunks,
            dirty_chunks,
        }
    }

    pub fn iter_chunks(&self) -> impl Iterator<Item = (&ChunkCoords, &Chunk)> {
        self.chunks.iter()
    }

    pub fn get_chunk(&self, chunk_coordinates: ChunkCoords) -> Option<&Chunk> {
        self.chunks.get(&chunk_coordinates)
    }

    // Пользователь функции обязан сам обновить соседние чанки.
    pub unsafe fn get_chunk_or_create(&mut self, chunk_coordinates: ChunkCoords) -> &Chunk {
        unsafe { self.get_chunk_mut_or_create(chunk_coordinates) }
    }

    pub fn get_chunk_mut(&mut self, chunk_coordinates: ChunkCoords) -> Option<&mut Chunk> {
        self.chunks.get_mut(&chunk_coordinates)
    }

    // Пользователь функции обязан сам обновить соседние чанки.
    pub unsafe fn get_chunk_mut_or_create(&mut self, chunk_coordinates: ChunkCoords) -> &mut Chunk {
        self.chunks.entry(chunk_coordinates).or_insert(Chunk::air())
    }

    fn dirt_chunk(&mut self, chunk_coordinates: ChunkCoords) {
        self.dirty_chunks.push(chunk_coordinates);
    }

    pub fn update_chunk_meshes(&mut self, renderer: &mut Renderer) {
        if self.dirty_chunks.len() > 0 {
            let start = Instant::now();
            while self.dirty_chunks.len() > 0 {
                let dirty_chunk_coords = self.dirty_chunks.pop().unwrap();

                let chunk_mesh = self.get_chunk_mesh(dirty_chunk_coords.clone());

                let chunk = self.get_chunk_mut(dirty_chunk_coords).unwrap();

                if let Some(old_id) = chunk.vram_slot_id {
                    renderer.unload(old_id);
                }

                let id = renderer
                    .load_chunk(chunk_mesh, dirty_chunk_coords.0.to_vec4_left().to_array())
                    .unwrap();
                chunk.vram_slot_id = Some(id);
            }
            println!("Обновление чанков: {} ms", start.elapsed().as_millis());
        }
    }

    pub fn get_chunk_mesh(&self, chunk_coordinates: ChunkCoords) -> BlockIndexedPrimitive {
        let central_chunk = self.chunks.get(&chunk_coordinates).unwrap();

        central_chunk.get_mesh()
    }

    pub fn get_block(&self, coords: GlobalCoords) -> Block {
        match self.get_chunk(coords.get_chunk()) {
            Some(chunk) => chunk.get_block(coords.get_local()),
            None => Block::default(),
        }
    }

    pub fn set_block(&mut self, coords: GlobalCoords, block: Block) {
        let (local_x, local_y, local_z) = coords.get_local().into(); // Игровые координаты 0..31
        let (chunk_x, chunk_y, chunk_z) = coords.get_chunk().into();

        // 1. Изменяем блок в ОФИЦИАЛЬНОМ чанке
        let chunk_coords = coords.get_chunk();
        // Используем твой метод (с unsafe или без), чтобы получить текущий чанк
        let chunk = unsafe { self.get_chunk_mut_or_create(chunk_coords) };
        chunk.set_block(coords.get_local(), block);
        self.dirt_chunk(chunk_coords); // Помечаем текущий чанк грязным

        // 2. СИНХРОНИЗАЦИЯ ВОРОТНИКОВ (Проверяем все 6 сторон по очереди)

        // --- ОСЬ X (Запад / Восток) ---
        if local_x == 0 {
            // Блок на левом краю чанка. Он является ВОСТОЧНЫМ воротником (индекс 33) для соседа СЛЕВА (-X)
            let neighbor_coords = (chunk_x - 1, chunk_y, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            // У соседа координата по X — это правый воротник (33)
            // Координаты Y и Z переводим во внутренний диапазон массива соседа (делаем +1)
            neighbor.set_block_raw(InternalCoords::new(33, local_y + 1, local_z + 1), block);
            self.dirt_chunk(neighbor_coords);
        } else if local_x == 31 {
            // Блок на правом краю чанка. Он является ЗАПАДНЫМ воротником (индекс 0) для соседа СПРАВА (+X)
            let neighbor_coords = (chunk_x + 1, chunk_y, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(0, local_y + 1, local_z + 1), block);
            self.dirt_chunk(neighbor_coords);
            // println!("Блок установлен на: {:?}", InternalCoords::from((0, local_y + 1, local_z + 1)));
        }

        // --- ОСЬ Y (Низ / Верх) ---
        if local_y == 0 {
            // Блок на самом дне чанка. Он является ВЕРХНИМ воротником (индекс 33) для соседа СНИЗУ (-Y)
            let neighbor_coords = (chunk_x, chunk_y - 1, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, 33, local_z + 1), block);
            self.dirt_chunk(neighbor_coords);
        } else if local_y == 31 {
            // Блок на самой крыше чанка. Он является НИЖНИМ воротником (индекс 0) для соседа СВЕРХУ (+Y)
            let neighbor_coords = (chunk_x, chunk_y + 1, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, 0, local_z + 1), block);
            self.dirt_chunk(neighbor_coords);
        }

        // --- ОСЬ Z (Север / Юг) ---
        if local_z == 0 {
            // Блок на северном краю чанка. Он является ЮЖНЫМ воротником (индекс 33) для соседа СЗАДИ (-Z)
            let neighbor_coords = (chunk_x, chunk_y, chunk_z - 1).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, local_y + 1, 33), block);
            self.dirt_chunk(neighbor_coords);
        } else if local_z == 31 {
            // Блок на южном краю чанка. Он является СЕВЕРНЫМ воротником (индекс 0) для соседа СПЕРЕДИ (+Z)
            let neighbor_coords = (chunk_x, chunk_y, chunk_z + 1).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, local_y + 1, 0), block);
            self.dirt_chunk(neighbor_coords);
        }
    }

    pub fn sync_borders_y(&mut self, pos_bottom: ChunkCoords, pos_top: ChunkCoords) {
        // Нам нужны ссылки на оба чанка одновременно.
        // В Rust нельзя взять две &mut ссылки из одной HashMap напрямую.
        // Поэтому используем безопасный метод get_many_mut (если доступен) или split_at_mut,
        // но проще всего временно достать один чанк, либо написать unsafe, либо по ID:

        // Предположим, у тебя есть способ получить мутабельные ссылки на оба чанка:
        let (chunk_bottom, chunk_top) = self.get_two_chunks_mut(pos_bottom, pos_top);

        // МЕГА-ОПТИМИЗАЦИЯ: Копируем целый слой 1156 блоков за ОДНУ команду!

        // 1. Наш ВЕРХНИЙ игровой слой (Y = 32) Чанка Нижнего
        //    копируется в НИЖНИЙ воротник (Y = 0) Чанка Верхнего
        let src_start = usize::from(InternalCoords::new(0, 32, 0));
        let src_end = src_start + InternalCoords::STRIDE_Y;
        let dst_start = usize::from(InternalCoords::new(0, 0, 0));

        chunk_top.data[dst_start..dst_start + InternalCoords::STRIDE_Y]
            .copy_from_slice(&chunk_bottom.data[src_start..src_end]);

        // 2. И наоборот: НИЖНИЙ игровой слой (Y = 1) Чанка Верхнего
        //    копируется в ВЕРХНИЙ воротник (Y = 33) Чанка Нижнего
        let src_start_2 = usize::from(InternalCoords::new(0, 1, 0));
        let src_end_2 = src_start_2 + InternalCoords::STRIDE_Y;
        let dst_start_2 = usize::from(InternalCoords::new(0, 33, 0));

        chunk_bottom.data[dst_start_2..dst_start_2 + InternalCoords::STRIDE_Y]
            .copy_from_slice(&chunk_top.data[src_start_2..src_end_2]);
    }

    /// Синхронизирует воротники по оси Z (СЕВЕР / СЮГ)
    /// Координата Z идет с шагом 34, поэтому копируем построчно вдоль оси X
    pub fn sync_borders_z(&mut self, pos_north: ChunkCoords, pos_south: ChunkCoords) {
        let (chunk_north, chunk_south) = self.get_two_chunks_mut(pos_north, pos_south);

        for y in 0..34 {
            // 1. Наш ЮЖНЫЙ игровой слой (Z = 32) Чанка Северного
            //    копируется в СЕВЕРНЫЙ воротник (Z = 0) Чанка Южного
            //    Линия вдоль оси X (34 блока) лежит в памяти ПОДРЯД, копируем слайсом!
            let src_idx = usize::from(InternalCoords::new(0, y, 32));
            let dst_idx = usize::from(InternalCoords::new(0, y, 0));
            chunk_south.data[dst_idx..dst_idx + 34]
                .copy_from_slice(&chunk_north.data[src_idx..src_idx + 34]);

            // 2. Наш СЕВЕРНЫЙ игровой слой (Z = 1) Чанка Южного
            //    копируется в ЮЖНЫЙ воротник (Z = 33) Чанка Северного
            let src_idx_2 = usize::from(InternalCoords::new(0, y, 1));
            let dst_idx_2 = usize::from(InternalCoords::new(0, y, 33));
            chunk_north.data[dst_idx_2..dst_idx_2 + 34]
                .copy_from_slice(&chunk_south.data[src_idx_2..src_idx_2 + 34]);
        }
    }

    /// Синхронизирует воротники по оси X (ВОСТОК / ЗАПАД)
    /// Координата X — это отдельные биты/байты, слайсами не взять, идем обычным циклом
    pub fn sync_borders_x(&mut self, pos_west: ChunkCoords, pos_east: ChunkCoords) {
        let (chunk_west, chunk_east) = self.get_two_chunks_mut(pos_west, pos_east);

        for y in 0..34 {
            for z in 0..34 {
                // ВОСТОЧНЫЙ игровой край (X = 32) Чанка Западного -> ЗАПАДНЫЙ воротник (X = 0) Чанка Восточного
                let src_idx = usize::from(InternalCoords::new(32, y, z));
                let dst_idx = usize::from(InternalCoords::new(0, y, z));
                chunk_east.data[dst_idx] = chunk_west.data[src_idx];

                // ЗАПАДНЫЙ игровой край (X = 1) Чанка Восточного -> ВОСТОЧНЫЙ воротник (X = 33) Чанка Западного
                let src_idx_2 = usize::from(InternalCoords::new(1, y, z));
                let dst_idx_2 = usize::from(InternalCoords::new(33, y, z));
                chunk_west.data[dst_idx_2] = chunk_east.data[src_idx_2];
            }
        }
    }

    fn get_two_chunks_mut(
        &mut self,
        pos_a: ChunkCoords,
        pos_b: ChunkCoords,
    ) -> (&mut Chunk, &mut Chunk) {
        assert!(pos_a != pos_b, "Попытка синхронизировать чанк сам с собой!");

        unsafe {
            let ptr_a = self.chunks.get_mut(&pos_a).unwrap() as *mut Chunk;
            let ptr_b = self.chunks.get_mut(&pos_b).unwrap() as *mut Chunk;
            (&mut *ptr_a, &mut *ptr_b)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}
