struct WordlCameraUniform {
    proj_matrix: mat4x4<f32>,
    view_rotation: mat4x4<f32>,
    camera_chunk: vec4<i32>,
    camera_in_chunk_position: vec4<f32>,
}

struct OutlineUniform {
    // Координаты целевого блока относительно чанка камеры
    relative_block_pos: vec4<f32>, 
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: WordlCameraUniform;
@group(0) @binding(1) var<uniform> outline: OutlineUniform;

struct VertexInput {
    @location(0) local_pos: vec3<f32>, // Сюда прилетят наши статические 0..1 координаты ребер
}

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    // Позиция ребра куба относительно камеры:
    // Позиция = Относительный_Блок + Локальная_Точка_Ребра - Позиция_Камеры_В_Чанке
    let camera_relative_pos = outline.relative_block_pos.xyz + in.local_pos - camera.camera_in_chunk_position.xyz;

    return camera.proj_matrix * camera.view_rotation * vec4<f32>(camera_relative_pos, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return outline.color; // Например, полупрозрачный черный или белый цвет
}
