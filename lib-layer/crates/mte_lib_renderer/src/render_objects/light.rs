#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniversalLightData {
    // XYZ — Позиция в мире (нужна для Point и Spot)
    pub position: [f32; 3],
    // W — Тип источника: 0.0 = Directional, 1.0 = Point, 2.0 = Spot
    pub type_: f32,
    // RGB — Цвет света.
    pub color: [f32; 3],
    // A — Интенсивность (яркость)
    pub intentsity: f32,
    // XYZ — Направление лучей (нужно для Directional и Spot)
    pub direction: [f32; 3],
    // W   — Радиус затухания света (для Point и Spot, дальше этого расстояния свет равен 0)
    pub range: f32,
    // X — Внутренний угол конуса прожектора (в радианах, где свет максимальный)
    // Y — Внешний угол конуса прожектора (где свет плавно угасает в ноль)
    pub spot_angles: [f32; 2],
    // Z — Индекс слоя в массиве карт теней (Texture Array Index), привязанный к этой лампе
    // W — Паддинг для выравнивания структуры по границе 16 байт
    pub shadow_idx: [f32; 2],
    // Матрица трансформации для расчета теней
    // Примечание: для Point Light (точечного) в идеале нужна CubeMap-матрица (6 штук),
    // но для простоты на старте Point Light часто делают без теней или используют Spot.
    pub view_projection: [[f32; 4]; 4],
}
