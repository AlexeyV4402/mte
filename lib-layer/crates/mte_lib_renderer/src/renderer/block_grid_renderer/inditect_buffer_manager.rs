use lib_core::alloc_helper::ConstPageAllocHelper;
use lib_core::math::vectors::custom::{PrecisePosition, PrecisePositionC32};
use wgpu::wgt::DrawIndexedIndirectArgs;
use wgpu::{Buffer, CommandEncoder, Queue};

use super::consts::{
    GLOBAL_BUFFER_INDEX_PER_SECTION, GLOBAL_BUFFER_SECTION_COUNT, GLOBAL_BUFFER_VERTEX_PER_SECTION, GLOBAL_INDEX_BUFFER_CAPACITY, GLOBAL_INDEX_BUFFER_SECTION_CAPACITY, GLOBAL_INDIRECT_BUFFER_CAPACITY, GLOBAL_MATRIX_BUFFER_CAPACITY, GLOBAL_VECTOR_BUFFER_CAPACITY, GLOBAL_VERTEX_BUFFER_CAPACITY, GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY
};
use crate::context::gpu_context::GpuContext;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;

#[repr(C)]
pub enum BufferType {
    Vertex = 0,
    Index = 1,
    Matrix = 2,
    Vector = 3,
}

const SECTION_CAPACITIES: [usize; 4] = [
    GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY,
    GLOBAL_INDEX_BUFFER_SECTION_CAPACITY,
    std::mem::size_of::<[[f32; 4]; 4]>(),
    std::mem::size_of::<[i32; 4]>(),
];

pub struct StagingBufferCommand {
    src_offset: u64,
    dst_offset: u64,
    length: u64,
    dst_type: usize,
}

pub struct IndirectBufferManager {
    pub gpu_staging_buffer: Buffer,
    pub cpu_staging_buffer: Vec<u8>,

    pub cpu_staging_queue: Vec<StagingBufferCommand>,

    pub gpu_indexed_indirect_buffer: Buffer,
    pub cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs>,

    global_buffers: [Buffer; 4],

    pub global_buffer_free_slots: Vec<usize>,
}

impl IndirectBufferManager {
    pub fn new(gpu_context: &GpuContext) -> Self {
        let gpu_staging_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: 1024 * 1024 * 1024,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_vertex_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: GLOBAL_VERTEX_BUFFER_CAPACITY as u64,
            mapped_at_creation: false,
        });

        let global_index_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            size: GLOBAL_INDEX_BUFFER_CAPACITY as u64,
            mapped_at_creation: false,
        });

        let global_matrix_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Matrix Buffer"),
            size: GLOBAL_MATRIX_BUFFER_CAPACITY as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_vector_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vector Buffer"),
            size: GLOBAL_VECTOR_BUFFER_CAPACITY as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs> =
            vec![DrawIndexedIndirectArgs::default(); GLOBAL_BUFFER_SECTION_COUNT];

        let gpu_indexed_indirect_buffer =
            gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Indexed Indirect Buffer"),
                usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
                size: GLOBAL_INDIRECT_BUFFER_CAPACITY as u64,
                mapped_at_creation: false,
            });

        Self {
            gpu_staging_buffer,
            cpu_staging_buffer: Vec::new(),
            cpu_staging_queue: Vec::new(),
            gpu_indexed_indirect_buffer,
            cpu_indexed_indirect_buffer,
            global_buffers: [
                global_vertex_buffer,
                global_index_buffer,
                global_matrix_buffer,
                global_vector_buffer,
            ],

            global_buffer_free_slots: (1..GLOBAL_BUFFER_SECTION_COUNT).collect(),
        }
    }

    pub fn load_chunk(
        &mut self,
        primitive: BlockIndexedPrimitive,
        vector: [i32; 4],
    ) -> anyhow::Result<usize> {
        if primitive.vertices.len() > GLOBAL_BUFFER_VERTEX_PER_SECTION as usize
            || primitive.indices.len() > GLOBAL_BUFFER_INDEX_PER_SECTION as usize
        {
            return Err(anyhow::anyhow!(
                "{} вершин дано;\n{} вершин допустимо;\n{} индексов дано;\n{} индексов допустимо;",
                primitive.vertices.len(),
                GLOBAL_BUFFER_VERTEX_PER_SECTION,
                primitive.indices.len(),
                GLOBAL_BUFFER_INDEX_PER_SECTION
            ));
        }

        let free = self
            .global_buffer_free_slots
            .pop()
            .ok_or(anyhow::anyhow!("Нет свободных слотов в VRAM"))?;

        self.write_with_vector(free, &primitive, &vector);

        Ok(free)
    }

    pub fn unload(&mut self, slot_id: usize) {
        self.global_buffer_free_slots.push(slot_id);

        self.cpu_indexed_indirect_buffer[slot_id] = DrawIndexedIndirectArgs {
            index_count: 0,
            instance_count: 0,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        };
    }

    pub fn prepare_buffers(&mut self, queue: &Queue, encoder: &mut CommandEncoder) {
        queue.write_buffer(
            &self.gpu_staging_buffer,
            0,
            bytemuck::cast_slice(&self.cpu_staging_buffer),
        );
        for command in self.cpu_staging_queue.drain(..) {
            encoder.copy_buffer_to_buffer(
                &self.gpu_staging_buffer,
                command.src_offset,
                &self.global_buffers[command.dst_type as usize],
                command.dst_offset,
                command.length,
            );
        }

        queue.write_buffer(
            &self.gpu_indexed_indirect_buffer,
            0,
            bytemuck::cast_slice(&self.cpu_indexed_indirect_buffer),
        );

        self.cpu_staging_buffer.clear();
    }

    pub fn get_global_buffer<const BUFFER_ID: usize>(&self) -> &Buffer {
        return &self.global_buffers[BUFFER_ID];
    }

    pub fn write_with_matrix(
        &mut self,
        slot_id: usize,
        primitive: &BlockIndexedPrimitive,
        matrix: &[[f32; 4]; 4],
    ) {
        // Нам нужно знать текущую длину буфера ДО добавления новых данных.
        // Для самого первого чанка в кадре это будет 0, для второго — конец геометрии первого.
        let mut current_offset = self.cpu_staging_buffer.len();

        // === ВЕРШИНЫ ===
        let v_bytes = bytemuck::cast_slice(&primitive.vertices);
        let v_len = v_bytes.len();

        self.write::<{ BufferType::Vertex as usize }>(slot_id, current_offset, v_bytes, v_len);

        current_offset += v_len; // Шагаем СТРОГО на размер вершин!

        // === ИНДЕКСЫ ===
        let i_bytes = bytemuck::cast_slice(&primitive.indices);
        let i_len = i_bytes.len();

        self.write::<{ BufferType::Index as usize }>(slot_id, current_offset, i_bytes, i_len);

        current_offset += i_len; // Шагаем СТРОГО на размер индексов!

        // === МАТРИЦА ===
        let m_bytes = bytemuck::cast_slice(matrix);
        let m_len = m_bytes.len();

        self.write::<{ BufferType::Matrix as usize }>(slot_id, current_offset, m_bytes, m_len);

        // Записываем команду отрисовки
        self.cpu_indexed_indirect_buffer[slot_id as usize] = DrawIndexedIndirectArgs {
            index_count: primitive.indices.len() as u32,
            instance_count: 1,
            first_index: slot_id as u32 * GLOBAL_BUFFER_INDEX_PER_SECTION,
            base_vertex: (slot_id as u32 * GLOBAL_BUFFER_VERTEX_PER_SECTION) as i32,
            first_instance: slot_id as u32,
        };
    }

    pub fn write_with_vector(
        &mut self,
        slot_id: usize,
        primitive: &BlockIndexedPrimitive,
        vector: &[i32; 4],
    ) {
        let mut current_offset = self.cpu_staging_buffer.len();

        // === ВЕРШИНЫ ===
        let v_bytes = bytemuck::cast_slice(&primitive.vertices);
        let v_len = v_bytes.len();

        self.write::<{ BufferType::Vertex as usize }>(slot_id, current_offset, v_bytes, v_len);

        current_offset += v_len; // Шагаем СТРОГО на размер вершин!

        // === ИНДЕКСЫ ===
        let i_bytes = bytemuck::cast_slice(&primitive.indices);
        let i_len = i_bytes.len();

        self.write::<{ BufferType::Index as usize }>(slot_id, current_offset, i_bytes, i_len);

        current_offset += i_len; // Шагаем СТРОГО на размер индексов!

        // === ВЕКТОР ===
        let v_bytes = bytemuck::cast_slice(vector);
        let v_len = v_bytes.len();

        self.write::<{ BufferType::Vector as usize }>(slot_id, current_offset, v_bytes, v_len);

        // Записываем команду отрисовки
        self.cpu_indexed_indirect_buffer[slot_id as usize] = DrawIndexedIndirectArgs {
            index_count: primitive.indices.len() as u32,
            instance_count: 1,
            first_index: slot_id as u32 * GLOBAL_BUFFER_INDEX_PER_SECTION,
            base_vertex: (slot_id as u32 * GLOBAL_BUFFER_VERTEX_PER_SECTION) as i32,
            first_instance: slot_id as u32,
        };
    }

    #[inline]
    pub fn write<const BUFFER_ID: usize>(
        &mut self,
        idx: usize,
        current_offset: usize,
        bytes: &[u8],
        len: usize,
    ) {
        let offset_bytes = idx * SECTION_CAPACITIES[BUFFER_ID];

        self.cpu_staging_buffer.extend_from_slice(bytes);

        self.cpu_staging_queue.push(StagingBufferCommand {
            src_offset: current_offset as u64,
            dst_offset: offset_bytes as u64,
            length: len as u64,
            dst_type: BUFFER_ID,
        });
    }
}

// pub struct IndirectBufferManager_ {
//     pub gpu_staging_buffer: Buffer,
//     pub cpu_staging_buffer: Vec<u8>,

//     pub cpu_staging_queue: Vec<StagingBufferCommand>,

//     pub gpu_indexed_indirect_buffer: Buffer,
//     pub cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs>,

//     pub vertex_buffer: PageBuffer,
//     pub index_buffer: PageBuffer,
//     pub matrix_buffer: SlotBuffer,
//     pub vector_buffer: SlotBuffer
// }

// impl IndirectBufferManager_ {

//     pub const ONE_VERTEX_BUFFER_PAGE_CAPACITY: u64 = 4 * 1024;
//     pub const ONE_VERTEX_BUFFER_PAGE_COUNT: u64 = 64 * 1024;
//     pub const ONE_VERTEX_BUFFER_CAPACITY: u64 = Self::ONE_VERTEX_BUFFER_PAGE_CAPACITY * Self::ONE_VERTEX_BUFFER_PAGE_COUNT;

//     pub const ONE_INDEX_BUFFER_PAGE_CAPACITY: u64 = 4 * 1024;
//     pub const ONE_INDEX_BUFFER_PAGE_COUNT: u64 = 64 * 1024;
//     pub const ONE_INDEX_BUFFER_CAPACITY: u64 = Self::ONE_INDEX_BUFFER_PAGE_CAPACITY * Self::ONE_INDEX_BUFFER_PAGE_COUNT;

//     pub const ONE_MATRIX_BUFFER_SLOTS_COUNT: u64 = 1;
//     pub const ONE_MATRIX_BUFFER_CAPACITY: u64 = Self::ONE_MATRIX_BUFFER_SLOTS_COUNT * (size_of::<[[f32; 4]; 4]>() as u64);

//     pub const ONE_VECTOR_BUFFER_SLOTS_COUNT: u64 = 256;
//     pub const ONE_VECTOR_BUFFER_CAPACITY: u64 = Self::ONE_VECTOR_BUFFER_SLOTS_COUNT * (size_of::<[i32; 4]>() as u64);

//     pub fn new(gpu_context: &GpuContext) -> Self {
//         let global_vertex_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Vertex Buffer"),
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//             size: Self::ONE_VERTEX_BUFFER_CAPACITY,
//             mapped_at_creation: false,
//         });

//         let vertex_buffer = PageBuffer {
//             gpu_buffers: vec![global_vertex_buffer],
//             alloc_helper: ConstPageAllocHelper {
//                 bitset: vec![0u64; (Self::ONE_VERTEX_BUFFER_CAPACITY / 64) as usize],
//                 total_sectors: (Self::ONE_VERTEX_BUFFER_CAPACITY / 64) as usize,
//             },
//         };

//         let global_index_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Index Buffer"),
//             usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
//             size: Self::ONE_INDEX_BUFFER_CAPACITY,
//             mapped_at_creation: false,
//         });

//         let index_buffer = PageBuffer {
//             gpu_buffers: vec![global_index_buffer],
//             alloc_helper: ConstPageAllocHelper {
//                 bitset: vec![0u64; (Self::ONE_INDEX_BUFFER_CAPACITY / 64) as usize],
//                 total_sectors: (Self::ONE_INDEX_BUFFER_CAPACITY / 64) as usize,
//             },
//         };

//         let global_matrix_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Matrix Buffer"),
//             size: Self::ONE_MATRIX_BUFFER_CAPACITY,
//             usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });

//         let matrix_buffer = SlotBuffer {
//             gpu_buffers: vec![global_matrix_buffer],
//             free_slots: (0..Self::ONE_MATRIX_BUFFER_SLOTS_COUNT).collect(),
//         };

//         let global_vector_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Vector Buffer"),
//             size: Self::ONE_VECTOR_BUFFER_CAPACITY,
//             usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });

//         let vector_buffer = SlotBuffer {
//             gpu_buffers: vec![global_vector_buffer],
//             free_slots: (0..Self::ONE_VECTOR_BUFFER_SLOTS_COUNT).collect(),
//         };

//         let gpu_staging_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("Staging Buffer"),
//             size: (Self::ONE_MATRIX_BUFFER_SLOTS_COUNT + Self::ONE_VECTOR_BUFFER_SLOTS_COUNT) * (size_of::<DrawIndexedIndirectArgs>() as u64),
//             usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });

//         let cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs> =
//             vec![DrawIndexedIndirectArgs::default(); GLOBAL_BUFFER_SECTION_COUNT];

//         let gpu_indexed_indirect_buffer =
//             gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
//                 label: Some("Indexed Indirect Buffer"),
//                 usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
//                 size: GLOBAL_INDIRECT_BUFFER_CAPACITY as u64,
//                 mapped_at_creation: false,
//             });

//         Self {
//             gpu_staging_buffer,
//             cpu_staging_buffer: Vec::new(),
//             cpu_staging_queue: Vec::new(),
//             gpu_indexed_indirect_buffer,
//             cpu_indexed_indirect_buffer,
//             vertex_buffer,
//             index_buffer,
//             matrix_buffer,
//             vector_buffer,
//         }
//     }

//     pub fn write_with_vector(&mut self, primitive: BlockIndexedPrimitive, vector: [i32; 4]) {
//         let mut current_offset = self.cpu_staging_buffer.len();

//         // === ВЕРШИНЫ ===
//         let v_bytes = bytemuck::cast_slice(&primitive.vertices);
//         let v_len = v_bytes.len();

//         self.cpu_staging_buffer.extend_from_slice(v_bytes);

//         let offset_pages = self.vertex_buffer.alloc_helper.alloc(v_len.div_ceil(Self::ONE_VERTEX_BUFFER_PAGE_CAPACITY as usize)) as u64;

//         self.cpu_staging_queue.push(StagingBufferCommand {
//             src_offset: current_offset as u64,
//             dst_offset: offset_pages as u64 * Self::ONE_VERTEX_BUFFER_PAGE_CAPACITY,
//             length: len as u64,
//             dst_type: BUFFER_ID,
//         });
//         // self.write::<{ BufferType::Vertex as usize }>(slot_id, current_offset, v_bytes, v_len);

//         current_offset += v_len; // Шагаем СТРОГО на размер вершин!

//         // === ИНДЕКСЫ ===
//         let i_bytes = bytemuck::cast_slice(&primitive.indices);
//         let i_len = i_bytes.len();

//         self.write::<{ BufferType::Index as usize }>(slot_id, current_offset, i_bytes, i_len);

//         current_offset += i_len; // Шагаем СТРОГО на размер индексов!

//         // === ВЕКТОР ===
//         let v_bytes = bytemuck::cast_slice(vector);
//         let v_len = v_bytes.len();

//         self.write::<{ BufferType::Vector as usize }>(slot_id, current_offset, v_bytes, v_len);

//         // Записываем команду отрисовки
//         self.cpu_indexed_indirect_buffer[slot_id as usize] = DrawIndexedIndirectArgs {
//             index_count: primitive.indices.len() as u32,
//             instance_count: 1,
//             first_index: slot_id as u32 * GLOBAL_BUFFER_INDEX_PER_SECTION,
//             base_vertex: (slot_id as u32 * GLOBAL_BUFFER_VERTEX_PER_SECTION) as i32,
//             first_instance: slot_id as u32,
//         };
//     }
// }

// pub struct PageBuffer {
//     gpu_buffers: Vec<wgpu::Buffer>,
//     alloc_helper: ConstPageAllocHelper
// }

// impl PageBuffer {

// }

// pub struct SlotBuffer {
//     gpu_buffers: Vec<wgpu::Buffer>,
//     free_slots: Vec<u64>
// }

// pub struct PageHandle {
//     pub page_start: u64,
//     pub page_end: u64
// }

// pub struct ChunkDataHandle {
//     vertices: PageHandle,
//     indices: PageHandle,
//     vector: u64
// }

// pub struct HandDataHandle {
//     vertices: PageHandle,
//     indices: PageHandle,
//     matrix: u64
// }
