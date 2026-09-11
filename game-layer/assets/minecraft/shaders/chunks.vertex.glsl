#version 460

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
};

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
    // В Vulkan/GLSL встроенная переменная gl_InstanceID заменяет instance_index из WGSL
    uint instance_idx = gl_InstanceIndex; 

    // 1. Распаковываем координаты (каждая по 6 бит)
    float local_x = float(in_packed_data & 0x3Fu);
    float local_y = float((in_packed_data >> 6u) & 0x3Fu);
    float local_z = float((in_packed_data >> 12u) & 0x3Fu);
    vec3 local_pos = vec3(local_x, local_y, local_z);
    
    // 2. Распаковываем метаданные
    uint side_id = (in_packed_data >> 18u) & 0x7u;
    uint block_type_id = (in_packed_data >> 23u) & 0xFFFu;
    
    out_material_id = get_final_texture_layer(block_type_id, side_id);

    // 3. Считаем позицию вершины относительно камеры (Твой фикс f32)
    ivec4 chunk_pos_4d = vectors[instance_idx];
    ivec3 chunk_diff_i32 = chunk_pos_4d.xyz - camera_uniform.camera_chunk.xyz;
    
    vec3 chunk_relative_base_pos = vec3(chunk_diff_i32) * 32.0;
    vec3 camera_relative_pos = chunk_relative_base_pos + local_pos - camera_uniform.camera_in_chunk_position.xyz;

    // 4. MVP трансформация
    vec4 view_pos = camera_uniform.view_rotation * vec4(camera_relative_pos, 1.0);
    gl_Position = camera_uniform.proj_matrix * view_pos;

    // 5. Расчет UV текстурных координат на основе стороны
    if (side_id == SIDE_TOP || side_id == SIDE_BOTTOM) {
        out_uv = vec2(local_pos.x, local_pos.z);
    } else if (side_id == SIDE_NORTH || side_id == SIDE_SOUTH) {
        out_uv = vec2(1.0 - local_pos.x, 1.0 - local_pos.y);
    } else {
        out_uv = vec2(local_pos.z, 1.0 - local_pos.y);
    }
}