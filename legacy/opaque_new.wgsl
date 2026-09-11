struct VertexInput {
    @location(0) packed_data: u32,
    @builtin(instance_index) instance_idx: u32, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material_id: u32,
};

struct BlockProperty {
    base_id: u32,
    profile_id: u32,
};

const SIDE_TOP: u32 = 0u;
const SIDE_BOTTOM: u32 = 1u;
const SIDE_NORTH: u32 = 2u;
const SIDE_SOUTH: u32 = 3u;
const SIDE_WEST: u32 = 4u;
const SIDE_EAST: u32 = 5u;

struct WorldCameraUniform {
    proj_matrix: mat4x4<f32>,
    view_rotation: mat4x4<f32>,
    camera_chunk: vec4<i32>,
    camera_in_chunk_position: vec4<f32>,
}

// Fragment
@group(0) @binding(0) var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(1) var s_diffuse: sampler;
// Vertex
@group(0) @binding(2) var<storage, read> block_properties: array<BlockProperty>;
@group(1) @binding(0) var<uniform> camera_uniform: WorldCameraUniform;
@group(2) @binding(0) var<storage, read> vectors: array<vec4<i32>>;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // 1. Распаковываем координаты (каждая по 6 бит)
    let local_x = f32(vertex.packed_data & 0x3Fu);
    let local_y = f32((vertex.packed_data >> 6u) & 0x3Fu);
    let local_z = f32((vertex.packed_data >> 12u) & 0x3Fu);
    let local_pos = vec3<f32>(local_x, local_y, local_z);
    
    // 2. Распаковываем метаданные
    let side_id = (vertex.packed_data >> 18u) & 0x7u;
    let block_type_id = (vertex.packed_data >> 23u) & 0xFFFu;
    let material_id = get_final_texture_layer(block_type_id, side_id);
    output.material_id = material_id;

    // 3. Считаем позицию вершины (MVP)
    let chunk_pos_4d = vectors[vertex.instance_idx];
    
    let chunk_diff_i32 = chunk_pos_4d.xyz - camera_uniform.camera_chunk.xyz;
    
    let chunk_relative_base_pos = vec3<f32>(chunk_diff_i32) * 32.0;

    let camera_relative_pos = chunk_relative_base_pos + local_pos - camera_uniform.camera_in_chunk_position.xyz;

    // 4. MVP трансформация
    let view_pos = camera_uniform.view_rotation * vec4<f32>(camera_relative_pos, 1.0);
    output.clip_position = camera_uniform.proj_matrix * view_pos;

    if (side_id == SIDE_TOP || side_id == SIDE_BOTTOM) {
        // Горизонтальные грани берут плоскость XZ
        output.uv = vec2<f32>(local_pos.x, local_pos.z);
    } else if (side_id == SIDE_NORTH || side_id == SIDE_SOUTH) {
        // Стены Север/Юг берут плоскость XY
        output.uv = vec2<f32>(1.0 - local_pos.x, 1.0 - local_pos.y);
    } else {
        // Стены Восток/Запад берут плоскость YZ
        output.uv = vec2<f32>(local_pos.z, 1.0 - local_pos.y);
    }

    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv, in.material_id);
}


const MAPPING_OFFSETS = array<u32, 36>(
    // 0: AllSides (Профиль 0) -> везде базовая текстура (+0)
    0u, 0u, 0u, 0u, 0u, 0u,

    // 1: TopBottomSides (Профиль 1) -> TOP = +0, BOTTOM = +1, боковины = +2
    0u, 1u, 2u, 2u, 2u, 2u,

    // 2: AxisAligned (Профиль 2) -> TOP/BOTTOM = +0, боковины = +1
    0u, 0u, 1u, 1u, 1u, 1u,

    // 3: OrientedFront (Профиль 3) -> NORTH (Лицо) = +1, остальные = +0
    0u, 0u, 1u, 0u, 0u, 0u,

    // 4: Column (Профиль 4, Бревно) -> торцы (TOP/BOTTOM) = +0, +1, боковины = +2
    0u, 1u, 2u, 2u, 2u, 2u,

    // 5: Individual (Профиль 5) -> каждая грань идет строго по порядку
    0u, 1u, 2u, 3u, 4u, 5u
);


fn get_final_texture_layer(block_type: u32, side_id: u32) -> u32 {
    let prop = block_properties[block_type];
    
    let flat_idx = (prop.profile_id * 6u) + side_id;
    
    let offset = MAPPING_OFFSETS[flat_idx];
    
    return prop.base_id + offset;
}