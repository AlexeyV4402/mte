struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var t_msdf: texture_2d<f32>;
@group(0) @binding(1) var s_msdf: sampler;

// Юниформы (передаются из движка)
struct TextUniforms {
    matrix: mat4x4<f32>,   // Матрица проекции/вида (MVP)
    color: vec4<f32>,      // Цвет текста
    px_range: f32,         // Тот самый range из генератора (например, 4.0)
};
@group(0) @binding(2) var<uniform> uniforms: TextUniforms;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = model.uv;
    out.clip_position = uniforms.matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// Функция поиска медианы для трех каналов MSDF
fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Выборка RGB значений расстояний из атласа
    let msd = textureSample(t_msdf, s_msdf, in.uv).rgb;
    
    // 2. Извлекаем чистое расстояние через медиану
    let sd = median(msd.r, msd.g, msd.b);
    
    // 3. Расчет экранного сглаживания (Anti-aliasing)
    // fwidth вычисляет, сколько пикселей текстуры помещается в один пиксель экрана
    let screen_px_distance = uniforms.px_range * (sd - 0.5);
    let opacity = clamp(screen_px_distance / fwidth(screen_px_distance) + 0.5, 0.0, 1.0);
    
    // Если opacity близко к 0, можно сделать discard пикселя для оптимизации глубины
    if (opacity < 0.01) {
        discard;
    }

    return vec4<f32>(uniforms.color.rgb, uniforms.color.a * opacity);
}