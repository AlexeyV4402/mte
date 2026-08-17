use wgpu::wgt::DrawIndexedIndirectArgs;
use wgpu::{Buffer, CommandEncoder, Queue};

use super::consts::{
    GLOBAL_BUFFER_INDEX_PER_SECTION, GLOBAL_BUFFER_SECTION_COUNT, GLOBAL_BUFFER_VERTEX_PER_SECTION, GLOBAL_INDEX_BUFFER_CAPACITY, GLOBAL_INDEX_BUFFER_SECTION_CAPACITY, GLOBAL_INDIRECT_BUFFER_CAPACITY, GLOBAL_MATRIX_BUFFER_CAPACITY, GLOBAL_VERTEX_BUFFER_CAPACITY, GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY
};
use crate::context::gpu_context::GpuContext;
use crate::renderer::block_grid_renderer::render_objects::primitive::BlockIndexedPrimitive;

enum BufferType {
    Vertex = 0,
    Index = 1,
    Matrix = 2,
}

pub struct StagingBufferCommand {
    src_offset: usize,
    dst_offset: usize,
    length: usize,
    dst_type: BufferType,
}

pub struct BufferManager {
    pub gpu_staging_buffer: Buffer,
    pub cpu_staging_buffer: Vec<u8>,

    pub cpu_staging_queue: Vec<StagingBufferCommand>,

    pub gpu_indexed_indirect_buffer: Buffer,
    pub cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs>,

    global_buffers: [Buffer; 3],

    pub global_buffer_free_slots: Vec<usize>,
}

impl BufferManager {
    pub fn new(gpu_context: &GpuContext) -> Self {
        let gpu_staging_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: 1024 * 1024 * 1024, // 64 МБ
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_model_matrix_buffer =
            gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Global Matrix Buffer"),
                size: GLOBAL_MATRIX_BUFFER_CAPACITY as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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

        let cpu_indexed_indirect_buffer: Vec<DrawIndexedIndirectArgs> =
            vec![DrawIndexedIndirectArgs::default(); GLOBAL_BUFFER_SECTION_COUNT];

        let gpu_indexed_indirect_buffer =
            gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Indirect Buffer"),
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
                global_model_matrix_buffer,
            ],

            global_buffer_free_slots: (1..GLOBAL_BUFFER_SECTION_COUNT).collect(),
        }
    }

    pub fn load_mesh(
        &mut self,
        primitive: BlockIndexedPrimitive,
        model_matrix: [[f32; 4]; 4],
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

        self.write_to_slot(free, &primitive, &model_matrix);

        Ok(free)
    }

    pub fn unload_mesh(&mut self, slot_id: usize) {
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
        while !self.cpu_staging_queue.is_empty() {
            let command = self.cpu_staging_queue.pop().unwrap();
            encoder.copy_buffer_to_buffer(
                &self.gpu_staging_buffer,
                command.src_offset as u64,
                &self.global_buffers[command.dst_type as usize],
                command.dst_offset as u64,
                command.length as u64,
            );
        }

        queue.write_buffer(
            &self.gpu_indexed_indirect_buffer,
            0,
            bytemuck::cast_slice(&self.cpu_indexed_indirect_buffer),
        );

        self.cpu_staging_buffer.clear();
    }

    pub fn get_global_vertex_buffer(&self) -> &Buffer {
        return &self.global_buffers[0];
    }

    pub fn get_global_index_buffer(&self) -> &Buffer {
        return &self.global_buffers[1];
    }

    pub fn get_global_model_matrix_buffer(&self) -> &Buffer {
        return &self.global_buffers[2];
    }

    pub fn write_to_slot(
        &mut self,
        slot_id: usize,
        primitive: &BlockIndexedPrimitive,
        model_matrix: &[[f32; 4]; 4],
    ) {
        let vertex_offset_bytes = slot_id * GLOBAL_VERTEX_BUFFER_SECTION_CAPACITY;
        let index_offset_bytes = slot_id * GLOBAL_INDEX_BUFFER_SECTION_CAPACITY;
        let matrix_offset_bytes = slot_id * std::mem::size_of::<[[f32; 4]; 4]>();

        // Нам нужно знать текущую длину буфера ДО добавления новых данных.
        // Для самого первого чанка в кадре это будет 0, для второго — конец геометрии первого.
        let mut current_offset = self.cpu_staging_buffer.len();

        // === ВЕРШИНЫ ===
        let v_bytes = bytemuck::cast_slice(&primitive.vertices);
        let v_len = v_bytes.len();
        self.cpu_staging_buffer.extend_from_slice(v_bytes);

        self.cpu_staging_queue.push(StagingBufferCommand {
            src_offset: current_offset,
            dst_offset: vertex_offset_bytes,
            length: v_len,
            dst_type: BufferType::Vertex,
        });
        current_offset += v_len; // Шагаем СТРОГО на размер вершин!

        // === ИНДЕКСЫ ===
        let i_bytes = bytemuck::cast_slice(&primitive.indices);
        let i_len = i_bytes.len();
        self.cpu_staging_buffer.extend_from_slice(i_bytes);

        self.cpu_staging_queue.push(StagingBufferCommand {
            src_offset: current_offset,
            dst_offset: index_offset_bytes,
            length: i_len,
            dst_type: BufferType::Index,
        });
        current_offset += i_len; // Шагаем СТРОГО на размер индексов!

        // === МАТРИЦА ===
        let m_bytes = bytemuck::cast_slice(model_matrix);
        let m_len = m_bytes.len();
        self.cpu_staging_buffer.extend_from_slice(m_bytes);

        self.cpu_staging_queue.push(StagingBufferCommand {
            src_offset: current_offset,
            dst_offset: matrix_offset_bytes,
            length: m_len,
            dst_type: BufferType::Matrix,
        });

        // Записываем команду отрисовки [7]
        self.cpu_indexed_indirect_buffer[slot_id] = DrawIndexedIndirectArgs {
            index_count: primitive.indices.len() as u32,
            instance_count: 1,
            first_index: slot_id as u32 * GLOBAL_BUFFER_INDEX_PER_SECTION, // [7]
            base_vertex: (slot_id as u32 * GLOBAL_BUFFER_VERTEX_PER_SECTION) as i32, // [5]
            first_instance: slot_id as u32,
        };
    }
}
