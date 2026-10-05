#version 460

#extension GL_ARB_shader_draw_parameters : enable

//#extension GL_EXT_nonuniform_qualifier : enable
//#extension GL_ARB_descriptor_indexing : enable

const uint SIDE_TOP = 0u;
const uint SIDE_BOTTOM = 1u;
const uint SIDE_NORTH = 2u;
const uint SIDE_SOUTH = 3u;
const uint SIDE_WEST = 4u;
const uint SIDE_EAST = 5u;

struct BlockProperty {
    uint base_id;
    uint profile_id;
};

struct WorldCameraUniform {
    mat4 proj_matrix;
    mat4 view_rotation;
    ivec4 camera_chunk;
    vec4 camera_in_chunk_position;
};

// --- Входные данные (Vertex Attributes) ---
layout(location = 0) in uint in_packed_data;

// --- Выходные данные для Фрагментного шейдера ---
layout(location = 0) out vec2 out_uv;
layout(location = 1) flat out uint out_material_id; 
// flat обязателен для целых чисел, чтобы видеокарта не пыталась их интерполировать между пикселями

// --- Буферы и Ресурсы (Descriptor Sets) ---
layout(set = 0, binding = 2, std430) readonly buffer BlockProperties {
    BlockProperty block_properties[];
};

layout(set = 1, binding = 0, std140) uniform CameraUniform {
    WorldCameraUniform camera_uniform;
};

layout(set = 2, binding = 0, std430) uniform Vectors {
    ivec4 vectors[256];
} vector_pools[1024];

layout(set = 3, binding = 0, std430) uniform Matrices {
    mat4 matrices[256];
} matrix_pools[1024];

// Глобальный массив смещений
const uint MAPPING_OFFSETS[36] = uint[](
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

// Функция расчета финального слоя текстуры
uint get_final_texture_layer(uint block_type, uint side_id) {
    BlockProperty prop = block_properties[block_type];
    uint flat_idx = (prop.profile_id * 6u) + side_id;
    uint offset = MAPPING_OFFSETS[flat_idx];
    return prop.base_id + offset;
}

void main() {
    uint packed_vector_id = gl_InstanceIndex;
    
    uint vector_slot_offset = packed_vector_id & 0x0000FFFFu;
    uint vector_buffer_idx = packed_vector_id >> 16u;

    ivec4 chunk_pos_4d = vector_pools[vector_buffer_idx].vectors[vector_slot_offset];

    uint packed_matrix_id = uint(chunk_pos_4d.w); 

    uint matrix_slot_offset = packed_matrix_id & 0x0000FFFFu;
    uint matrix_buffer_idx = packed_matrix_id >> 16u;

    mat4 ship_rotation = matrix_pools[matrix_buffer_idx].matrices[matrix_slot_offset];

    float local_x = float(in_packed_data & 0x3Fu);
    float local_y = float((in_packed_data >> 6u) & 0x3Fu);
    float local_z = float((in_packed_data >> 12u) & 0x3Fu);
    vec3 local_pos = vec3(local_x, local_y, local_z);
    
    // 2. Распаковываем метаданные
    uint side_id = (in_packed_data >> 18u) & 0x7u;
    uint block_type_id = (in_packed_data >> 23u) & 0xFFFu;
    
    out_material_id = get_final_texture_layer(block_type_id, side_id);

    


    ivec3 chunk_diff_i32 = chunk_pos_4d.xyz - camera_uniform.camera_chunk.xyz;
    vec3 chunk_relative_base_pos = vec3(chunk_diff_i32) * 32.0;

    vec3 ship_space_pos = (ship_rotation * vec4(local_pos, 1.0)).xyz;

    vec3 camera_relative_pos = chunk_relative_base_pos + ship_space_pos - camera_uniform.camera_in_chunk_position.xyz;

    vec4 view_pos = camera_uniform.view_rotation * vec4(camera_relative_pos, 1.0);
    gl_Position = camera_uniform.proj_matrix * view_pos;



    // 6. Расчет UV текстурных координат на основе стороны
    if (side_id == SIDE_TOP || side_id == SIDE_BOTTOM) {
        out_uv = vec2(local_pos.x, local_pos.z);
    } else if (side_id == SIDE_NORTH || side_id == SIDE_SOUTH) {
        out_uv = vec2(1.0 - local_pos.x, 1.0 - local_pos.y);
    } else {
        out_uv = vec2(local_pos.z, 1.0 - local_pos.y);
    }
}