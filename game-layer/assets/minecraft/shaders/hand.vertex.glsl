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

// --- Входные данные (Vertex Attributes) ---
layout(location = 0) in uint in_packed_data;

// --- Выходные данные для Фрагментного шейдера ---
layout(location = 0) out vec2 out_uv;
layout(location = 1) flat out uint out_material_id;

// --- Буферы ресурсов (Descriptor Sets) ---
layout(set = 0, binding = 2, std430) readonly buffer BlockProperties {
    BlockProperty block_properties[];
};

layout(set = 1, binding = 0, std140) uniform ProjUniform {
    mat4 proj;
};

layout(set = 2, binding = 0, std430) uniform Matrices {
    mat4 matrices[1];
};

// Глобальный массив смещений
const uint MAPPING_OFFSETS[36] = uint[](
    // 0: AllSides (Профиль 0) -> везде базовая текстура (+0)
    0u, 0u, 0u, 0u, 0u, 0u,
    // 1: TopBottomSides (Профиль 2) -> TOP = +0, BOTTOM = +1, боковины = +2
    0u, 1u, 2u, 2u, 2u, 2u,
    // 2: AxisAligned (Профиль 1) -> TOP/BOTTOM = +0, боковины = +1
    0u, 0u, 1u, 1u, 1u, 1u,
    // 3: OrientedFront (Профиль 3) -> NORTH (Лицо) = +1, остальные = +0
    0u, 0u, 1u, 0u, 0u, 0u,
    // 4: Column (Профиль 4, Бревно) -> торцы (TOP/BOTTOM) = +0, +1, боковины = +2
    0u, 1u, 2u, 2u, 2u, 2u,
    // 5: Individual (Профиль 5) -> каждая грань идет строго по порядку
    0u, 1u, 2u, 3u, 4u, 5u
);

uint get_final_texture_layer(uint block_type, uint side_id) {
    BlockProperty prop = block_properties[block_type];
    uint flat_idx = (prop.profile_id * 6u) + side_id;
    uint offset = MAPPING_OFFSETS[flat_idx];
    return prop.base_id + offset;
}

void main() {
    // Встроенная переменная для ID инстанса в Vulkan GLSL
    uint instance_idx = gl_InstanceIndex; 

    // 1. Распаковываем локальные координаты куба (0 или 1)
    float local_x = float(in_packed_data & 0x3Fu);
    float local_y = float((in_packed_data >> 6u) & 0x3Fu);
    float local_z = float((in_packed_data >> 12u) & 0x3Fu);
    
    // 2. Распаковываем метаданные
    uint side_id = (in_packed_data >> 18u) & 0x7u;
    uint block_type_id = (in_packed_data >> 23u) & 0xFFFu;
    
    out_material_id = get_final_texture_layer(block_type_id, side_id);

    // 3. Расчет позиции через матрицу инстанса и проекцию
    mat4 model_matrix = matrices[instance_idx];
    gl_Position = proj * model_matrix * vec4(local_x, local_y, local_z, 1.0);

    // 4. Считаем UV по локальным координатам, чтобы трипланарка не плыла
    if (side_id == SIDE_TOP || side_id == SIDE_BOTTOM) {
        out_uv = vec2(local_x, local_z);
    } else if (side_id == SIDE_NORTH || side_id == SIDE_SOUTH) {
        out_uv = vec2(1.0 - local_x, 1.0 - local_y);
    } else {
        out_uv = vec2(local_z, 1.0 - local_y);
    }
}