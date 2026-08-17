// // Vertex shader
// struct CameraUniform {
//     view_proj: mat4x4<f32>,
// };
// @group(1) @binding(0) // 1.
// var<uniform> camera: CameraUniform;

// struct VertexInput {
//     @location(0) position: vec3<f32>,
//     @location(1) tex_coords: vec2<f32>,
// }

// struct VertexOutput {
//     @builtin(position) clip_position: vec4<f32>,
//     @location(0) tex_coords: vec2<f32>,
// }

// @vertex
// fn vs_main(
//     model: VertexInput,
// ) -> VertexOutput {
//     var out: VertexOutput;
//     out.tex_coords = model.tex_coords;
//     out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
//     // out.clip_position = vec4<f32>(model.position, 1.0);
//     return out;
// }

// // Fragment shader

// @group(0) @binding(0)
// var t_diffuse: texture_2d<f32>;
// @group(0) @binding(1)
// var s_diffuse: sampler;

// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//     return textureSample(t_diffuse, s_diffuse, in.tex_coords);
//     // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
// }



struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @builtin(instance_index) instance_idx: u32, // НОВОЕ: Видеокарта сама передаст сюда наш slot_id!
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Матрица камеры (View-Projection) по-прежнему в Uniform
@group(1) @binding(0) var<uniform> view_proj: mat4x4<f32>;
// Подключаем наш гигантский Storage Buffer со всеми матрицами мира
@group(2) @binding(0) var<storage, read> model_matrices: array<mat4x4<f32>>;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Достаем матрицу именно для этого чанка по его instance_idx
    let model_matrix = model_matrices[model.instance_idx];
    
    // Сначала переводим локальные координаты вершины (0..32) в мировые (умножаем на матрицу модели)
    // А затем проецируем на камеру (умножаем на view_proj)
    let world_position = model_matrix * vec4<f32>(model.position, 1.0);
    out.clip_position = view_proj * world_position;
    
    out.uv = model.uv;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.uv);
    // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}