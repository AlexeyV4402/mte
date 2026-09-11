use std::time::Instant;
use std::{array, mem};

use lib_renderer::renderer::block_grid_renderer::Renderer;
use lib_renderer::renderer::block_grid_renderer::backend::vulkan_backend::renderer::VkBackend;
use lib_renderer::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashMap;

use crate::types::blocks::block::Block;
use crate::types::chunk::Chunk;
use crate::types::coordinates::core::{ChunkCoords, GlobalCoords, InternalCoords};
use crate::utils::save_manager::{self, SaveManager};
use crate::world_generator::WorldGenerator;

pub struct Dimension {
    chunks: FxHashMap<ChunkCoords, Chunk>,
    show_queue: Vec<ChunkCoords>,
    hide_queue: Vec<ChunkCoords>,
    prepare_queue: Vec<ChunkCoords>,
    unload_queue: Vec<ChunkCoords>,
}

impl Dimension {
    pub fn new() -> Self {
        Self {
            chunks: Default::default(),
            show_queue: Default::default(),
            hide_queue: Default::default(),
            prepare_queue: Default::default(),
            unload_queue: Default::default(),
        }
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

    pub fn show_chunk(&mut self, chunk_coordinates: ChunkCoords) {
        self.show_queue.push(chunk_coordinates);
    }

    pub fn hide_chunk(&mut self, chunk_coordinates: ChunkCoords) {
        self.hide_queue.push(chunk_coordinates);
    }

    pub fn prepare_chunk(&mut self, coords: ChunkCoords) {
        self.prepare_queue.push(coords);
    }

    pub fn unload_chunk(&mut self, coords: ChunkCoords) {
        self.unload_queue.push(coords);
    }

    pub fn prepare_chunks<G: WorldGenerator>(&mut self, generator: &G, save_manager: &SaveManager) {
        let queue = mem::replace(&mut self.prepare_queue, Vec::with_capacity(32));

        if queue.is_empty() {
            return;
        }

        let chunks = save_manager.load_exist(queue);

        let mut generate_queue = Vec::with_capacity(32);

        for (chunk_coords, chunk_data) in chunks {
            match chunk_data {
                Some(data) => {
                    let _ = &self.chunks.insert(chunk_coords, data);
                }
                None => generate_queue.push(chunk_coords),
            }
        }

        let mut generated = Vec::with_capacity(generate_queue.len());

        generate_queue
            .into_par_iter()
            .map(|coords| -> (ChunkCoords, Chunk) { (coords, generator.generate_chunk(coords)) })
            .collect_into_vec(&mut generated);

        self.chunks.extend(generated.into_iter());
    }

    pub fn save_chunks(&mut self, save_manager: &mut SaveManager) {
        if self.unload_queue.is_empty() {
            return;
        }

        let queue = mem::take(&mut self.unload_queue);

        let chunks: Vec<(ChunkCoords, Chunk)> = queue
            .into_iter()
            .filter_map(|coords| {
                if let Some(chunk) = self.chunks.remove(&coords) {
                    if chunk.is_changed {
                        Some((coords, chunk))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if !chunks.is_empty() {
            save_manager.save_chunks(chunks);
        }
    }

    pub fn update_chunk_meshes(&mut self, renderer: &mut VkBackend) {
        if self.show_queue.len() > 0 {
            // let start = Instant::now();

            let mut queue = mem::replace(&mut self.show_queue, Vec::with_capacity(32));
            queue.drain(..).for_each(|dirty_chunk_coords| {
                let chunk_mesh = self.get_chunk_mesh(dirty_chunk_coords.clone());

                if let Some(chunk) = self.get_chunk_mut(dirty_chunk_coords) {
                    if let Some(old_id) = chunk.vram_slot_id {
                        renderer.unload_chunk(old_id);
                    }

                    let id = renderer
                        .load_chunk(chunk_mesh, dirty_chunk_coords.0.to_vec4_left().to_array());
                    chunk.vram_slot_id = id;
                }
            });

            let mut queue = mem::replace(&mut self.hide_queue, Vec::with_capacity(32));
            queue.drain(..).for_each(|dirty_chunk_coords| {
                if let Some(chunk) = self.get_chunk_mut(dirty_chunk_coords) {
                    if let Some(old_id) = chunk.vram_slot_id {
                        renderer.unload_chunk(old_id);
                    }
                    chunk.vram_slot_id = None;
                }
            });

            // println!("Обновление чанков: {} ms", start.elapsed().as_millis());
        };
    }

    pub fn get_chunk_mesh(&self, chunk_coordinates: ChunkCoords) -> BlockIndexedPrimitive {
        if let Some(chunk) = self.chunks.get(&chunk_coordinates) {
            chunk.get_mesh()
        } else {
            BlockIndexedPrimitive {
                vertices: Vec::new(),
                indices: Vec::new(),
            }
        }
    }

    pub fn get_block_loaded(&self, coords: GlobalCoords) -> Block {
        match self.get_chunk(coords.get_chunk()) {
            Some(chunk) => chunk.get_block(coords.get_local()),
            None => Block::default(),
        }
    }

    /// Функция работает нормально только если соседний чанк в прогрузе
    pub fn set_block_loaded(&mut self, coords: GlobalCoords, block: Block) {
        let (local_x, local_y, local_z) = coords.get_local().into(); // Игровые координаты 0..31
        let (chunk_x, chunk_y, chunk_z) = coords.get_chunk().into();

        // 1. Изменяем блок в ОФИЦИАЛЬНОМ чанке
        let chunk_coords = coords.get_chunk();
        // Используем твой метод (с unsafe или без), чтобы получить текущий чанк
        let chunk = unsafe { self.get_chunk_mut_or_create(chunk_coords) };
        chunk.set_block(coords.get_local(), block);
        self.show_chunk(chunk_coords); // Помечаем текущий чанк грязным

        // 2. СИНХРОНИЗАЦИЯ ВОРОТНИКОВ (Проверяем все 6 сторон по очереди)

        // --- ОСЬ X (Запад / Восток) ---
        if local_x == 0 {
            // Блок на левом краю чанка. Он является ВОСТОЧНЫМ воротником (индекс 33) для соседа СЛЕВА (-X)
            let neighbor_coords = (chunk_x - 1, chunk_y, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            // У соседа координата по X — это правый воротник (33)
            // Координаты Y и Z переводим во внутренний диапазон массива соседа (делаем +1)
            neighbor.set_block_raw(InternalCoords::new(33, local_y + 1, local_z + 1), block);
            self.show_chunk(neighbor_coords);
        } else if local_x == 31 {
            // Блок на правом краю чанка. Он является ЗАПАДНЫМ воротником (индекс 0) для соседа СПРАВА (+X)
            let neighbor_coords = (chunk_x + 1, chunk_y, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(0, local_y + 1, local_z + 1), block);
            self.show_chunk(neighbor_coords);
            // println!("Блок установлен на: {:?}", InternalCoords::from((0, local_y + 1, local_z + 1)));
        }

        // --- ОСЬ Y (Низ / Верх) ---
        if local_y == 0 {
            // Блок на самом дне чанка. Он является ВЕРХНИМ воротником (индекс 33) для соседа СНИЗУ (-Y)
            let neighbor_coords = (chunk_x, chunk_y - 1, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, 33, local_z + 1), block);
            self.show_chunk(neighbor_coords);
        } else if local_y == 31 {
            // Блок на самой крыше чанка. Он является НИЖНИМ воротником (индекс 0) для соседа СВЕРХУ (+Y)
            let neighbor_coords = (chunk_x, chunk_y + 1, chunk_z).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, 0, local_z + 1), block);
            self.show_chunk(neighbor_coords);
        }

        // --- ОСЬ Z (Север / Юг) ---
        if local_z == 0 {
            // Блок на северном краю чанка. Он является ЮЖНЫМ воротником (индекс 33) для соседа СЗАДИ (-Z)
            let neighbor_coords = (chunk_x, chunk_y, chunk_z - 1).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, local_y + 1, 33), block);
            self.show_chunk(neighbor_coords);
        } else if local_z == 31 {
            // Блок на южном краю чанка. Он является СЕВЕРНЫМ воротником (индекс 0) для соседа СПЕРЕДИ (+Z)
            let neighbor_coords = (chunk_x, chunk_y, chunk_z + 1).into();
            let neighbor = unsafe { self.get_chunk_mut_or_create(neighbor_coords) };
            neighbor.set_block_raw(InternalCoords::new(local_x + 1, local_y + 1, 0), block);
            self.show_chunk(neighbor_coords);
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
