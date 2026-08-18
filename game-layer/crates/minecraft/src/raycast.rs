use crate::types::coordinates::core::GlobalCoords;
use crate::types::dimension::Dimension;

pub struct RaycastResult {
    /// Глобальные координаты блока, в который упёрся луч (чтобы сломать)
    pub target_block: GlobalCoords,
    /// Глобальные координаты пустой ячейки ПЕРЕД этим блоком (чтобы поставить новый)
    pub previous_block: GlobalCoords,
}

/// origin: (f32, f32, f32) — мировые координаты камеры
/// direction: (f32, f32, f32) — нормализованный вектор взгляда (куда смотрим)
/// max_distance: f32 — дальность в блоках (например, 5.0 блоков)
pub fn raycast(
    dimension: &Dimension,
    origin: (f32, f32, f32),
    direction: (f32, f32, f32),
    max_distance: f32,
) -> Option<RaycastResult> {
    let (start_x, start_y, start_z) = origin;
    let (dir_x, dir_y, dir_z) = direction;

    // 1. Текущая целочисленная ячейка, где находится старт луча
    let mut cx = start_x.floor() as i64;
    let mut cy = start_y.floor() as i64;
    let mut cz = start_z.floor() as i64;

    // 2. Направление шага по осям (+1 или -1)
    let step_x = if dir_x > 0.0 { 1 } else { -1 };
    let step_y = if dir_y > 0.0 { 1 } else { -1 };
    let step_z = if dir_z > 0.0 { 1 } else { -1 };

    // 3. Сколько расстояния луча нужно пройти, чтобы преодолеть размер ровно в 1 воксель
    let t_delta_x = if dir_x != 0.0 {
        (1.0 / dir_x).abs()
    } else {
        f32::MAX
    };
    let t_delta_y = if dir_y != 0.0 {
        (1.0 / dir_y).abs()
    } else {
        f32::MAX
    };
    let t_delta_z = if dir_z != 0.0 {
        (1.0 / dir_z).abs()
    } else {
        f32::MAX
    };

    // 4. Расстояние луча до ближайшей следующей границы вокселя по каждой оси
    let mut t_max_x = if dir_x > 0.0 {
        (cx as f32 + 1.0 - start_x) * t_delta_x
    } else {
        (start_x - cx as f32) * t_delta_x
    };
    let mut t_max_y = if dir_y > 0.0 {
        (cy as f32 + 1.0 - start_y) * t_delta_y
    } else {
        (start_y - cy as f32) * t_delta_y
    };
    let mut t_max_z = if dir_z > 0.0 {
        (cz as f32 + 1.0 - start_z) * t_delta_z
    } else {
        (start_z - cz as f32) * t_delta_z
    };

    // Храним предыдущие координаты, чтобы знать, откуда луч "прилетел" (нужно для установки блоков)
    let mut prev_x = cx;
    let mut prev_y = cy;
    let mut prev_z = cz;

    let mut current_distance = 0.0;

    // Цикл шагов луча по сетке
    while current_distance < max_distance {
        // Проверяем, твердый ли блок в текущей ячейке
        let current_coords = GlobalCoords::new(cx, cy, cz);
        if dimension.get_block(current_coords).get_type().is_solid() {
            return Some(RaycastResult {
                target_block: current_coords,
                previous_block: GlobalCoords::new(prev_x, prev_y, prev_z),
            });
        }

        // Сохраняем текущие координаты как "предыдущие" перед следующим шагом
        prev_x = cx;
        prev_y = cy;
        prev_z = cz;

        // Магия Аманатидеса-Ву: выбираем ось с минимальным расстоянием до следующей грани куба
        // и делаем по ней ровно один шаг. Никаких условных ветвлений внутри!
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                current_distance = t_max_x;
                t_max_x += t_delta_x;
                cx += step_x;
            } else {
                current_distance = t_max_z;
                t_max_z += t_delta_z;
                cz += step_z;
            }
        } else {
            if t_max_y < t_max_z {
                current_distance = t_max_y;
                t_max_y += t_delta_y;
                cy += step_y;
            } else {
                current_distance = t_max_z;
                t_max_z += t_delta_z;
                cz += step_z;
            }
        }
    }

    None // Луч улетел в пустоту и ничего не задел
}
