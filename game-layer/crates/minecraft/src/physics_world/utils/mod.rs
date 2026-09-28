use glam::Vec3;

pub fn ray_intersects_aabb(
    ray_origin: Vec3,
    ray_dir: Vec3,
    box_min: Vec3,
    box_max: Vec3,
) -> Option<f32> {
    // 1. Считаем инверсию направления луча (1.0 / dir), чтобы заменить дорогое деление на умножение.
    // Если какая-то координата dir равна 0.0 (луч параллелен оси), Rust вернет Infinity.
    // Математика ниже это корректно обработает без падения программы.
    let inv_dir = Vec3::new(1.0 / ray_dir.x, 1.0 / ray_dir.y, 1.0 / ray_dir.z);

    // 2. Находим точки пересечения с плоскостями X
    let t1 = (box_min.x - ray_origin.x) * inv_dir.x;
    let t2 = (box_max.x - ray_origin.x) * inv_dir.x;
    // Определяем, где был вход, а где выход по оси X
    let tmin_x = t1.min(t2);
    let tmax_x = t1.max(t2);

    // 3. Находим точки пересечения с плоскостями Y
    let t3 = (box_min.y - ray_origin.y) * inv_dir.y;
    let t4 = (box_max.y - ray_origin.y) * inv_dir.y;
    let tmin_y = t3.min(t4);
    let tmax_y = t3.max(t4);

    // 4. Находим точки пересечения с плоскостями Z
    let t5 = (box_min.z - ray_origin.z) * inv_dir.z;
    let t6 = (box_max.z - ray_origin.z) * inv_dir.z;
    let tmin_z = t5.min(t6);
    let tmax_z = t5.max(t6);

    // 5. Находим общее время входа (самый поздний вход из всех трех осей)
    // и общее время выхода (самый ранний выход из всех трех осей)
    let tmin = tmin_x.max(tmin_y).max(tmin_z);
    let tmax = tmax_x.min(tmax_y).min(tmax_z);

    // --- Условия пролета мимо ---
    // Если tmin > tmax, это значит, что луч вышел из одной плоскости (например, по X)
    // еще до того, как успел войти в другую (по Y). То есть пролетел мимо.
    // Если tmax < 0.0, значит коробка находится позади луча (сзади игрока).
    if tmin > tmax || tmax < 0.0 {
        return None;
    }

    // Если tmin < 0.0, значит tmin позади нас, а tmax впереди.
    // Это физически означает, что игрок/камера находится ПРЯМО ВНУТРИ коробки!
    if tmin < 0.0 {
        // println!("dd");
        return Some(0.0); // Пересечение прямо на старте
    }

    // Возвращаем честное расстояние до грани коробки
    Some(tmin)
}
