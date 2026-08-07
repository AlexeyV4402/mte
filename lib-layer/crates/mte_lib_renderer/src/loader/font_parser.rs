use std::fs::File;
use std::io::Read;

use glam::Vec2;
use image::{ImageBuffer, Rgb};
use ttf_parser::{Face, OutlineBuilder};

const RED: u8 = 1;
const GREEN: u8 = 2;
const BLUE: u8 = 4;
const COLORS: [u8; 3] = [RED, GREEN, BLUE];

pub struct Line {
    pub start: Vec2,
    pub end: Vec2,
    pub color: u8,
}

pub struct Contour {
    pub points: Vec<Vec2>,
}

pub struct GlyphGeometry {
    pub contours: Vec<Contour>,
}

struct MyGeometryBuilder {
    pub glyph: GlyphGeometry,
    pub tolerance: f32,
    current_start: Vec2, // Запоминаем, куда был move_to для замыкания
    current_pos: Vec2,   // Запоминаем текущую позицию пера
}

fn flatten_quad_bezier(p0: Vec2, p1: Vec2, p2: Vec2, tolerance: f32, points: &mut Vec<Vec2>) {
    // 1. Находим расстояние d от управляющей точки p1 до прямой p0-p2
    let base = p2 - p0;
    let len_sq = base.length_squared();

    let d = if len_sq < 0.0001 {
        // Если p0 и p2 практически совпали
        (p1 - p0).length()
    } else {
        // Расстояние от точки до прямой через векторное произведение
        // Формула: |cross(base, p1 - p0)| / length(base)
        let v = p1 - p0;
        let cross_product = base.x * v.y - base.y * v.x;
        cross_product.abs() / len_sq.sqrt()
    };

    // 2. Если кривая достаточно плоская — останавливаемся и проводим линию
    if d <= tolerance {
        points.push(p2);
        return;
    }

    // 3. Считаем новые точки через встроенный .lerp() на f32-векторах!
    // t = 0.5 — это ровно середина отрезков
    let a = p0.lerp(p1, 0.5);
    let b = p1.lerp(p2, 0.5);
    let c = a.lerp(b, 0.5);

    // 4. Рекурсивный спуск для левой и правой половин кривой
    flatten_quad_bezier(p0, a, c, tolerance, points);
    flatten_quad_bezier(c, b, p2, tolerance, points);
}

impl OutlineBuilder for MyGeometryBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = Vec2::new(x, y);
        self.current_start = p;
        self.current_pos = p;
        // Начинаем новый контур и сразу кладем в него стартовую точку
        self.glyph.contours.push(Contour { points: vec![p] });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = Vec2::new(x, y);
        if let Some(contour) = self.glyph.contours.last_mut() {
            contour.points.push(p);
        }
        self.current_pos = p;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let p1 = Vec2::new(x1, y1);
        let p2 = Vec2::new(x, y);

        if let Some(contour) = self.glyph.contours.last_mut() {
            // Наш рекурсивный алгоритм Де Кастельжо из прошлых шагов
            // Он сам добавит все промежуточные точки и финальную p2 в конец вектора
            flatten_quad_bezier(
                self.current_pos,
                p1,
                p2,
                self.tolerance,
                &mut contour.points,
            );
        }
        self.current_pos = p2;
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        // Если планируете только .ttf, здесь можно оставить пустым или panic
    }

    fn close(&mut self) {
        if let Some(contour) = self.glyph.contours.last_mut() {
            // Проверяем: если последняя точка не совпадает со стартовой,
            // спецификация требует неявно их соединить.
            if self.current_pos != self.current_start {
                contour.points.push(self.current_start);
            }
        }
    }
}

// Функция генерации MSDF для одной буквы
pub fn generate_msdf_image(
    lines: &[Line],
    atlas_width: u32,
    atlas_height: u32,
    units_per_em: f32,         // Например, 2048.0 из ttf-parser
    font_size_in_texture: f32, // Например, 48.0 (размер буквы в пикселях)
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    // 1. Создаем пустой RGB буфер с помощью крейта image
    let mut img_buffer = ImageBuffer::new(atlas_width, atlas_height);

    // Коэффициент перевода: Пиксели текстуры -> Единицы шрифта
    let scale = font_size_in_texture / units_per_em;

    // Радиус спада полей в пикселях текстуры (рекомендуемое значение от 4.0 до 8.0)
    let px_range = 4.0;

    // 2. Основной цикл по каждому пикселю будущей картинки
    for (x_tex, y_tex, pixel) in img_buffer.enumerate_pixels_mut() {
        // Переводим координату центра пикселя из пикселей в пространство Font Units
        // Примечание: y_tex инвертируем (atlas_height - 1 - y_tex), так как в TTF ось Y растет вверх,
        // а в картинках image и на экранах — вниз.
        let p_tex = Vec2::new(x_tex as f32 + 0.5, (atlas_height - 1 - y_tex) as f32 + 0.5);
        let p_font = p_tex / scale;

        // --- ПОДЗАДАЧА А: Алгоритм Winding Number (Определение знака) ---
        let mut winding = 0;
        for line in lines {
            let a = line.start;
            let b = line.end;

            // Проверка пересечения горизонтального луча, идущего вправо из p_font
            if a.y <= p_font.y {
                if b.y > p_font.y {
                    // Линия идет вверх. Проверяем, находится ли пересечение правее точки
                    let intersect_x = a.x + (p_font.y - a.y) * (b.x - a.x) / (b.y - a.y);
                    if intersect_x > p_font.x {
                        winding += 1;
                    }
                }
            } else if b.y <= p_font.y {
                // Линия идет вниз
                if a.y > p_font.y {
                    let intersect_x = a.x + (p_font.y - a.y) * (b.x - a.x) / (b.y - a.y);
                    if intersect_x > p_font.x {
                        winding -= 1;
                    }
                }
            }
        }
        let is_inside = winding != 0;

        // --- ПОДЗАДАЧА Б: Поиск минимальных расстояний по каналам ---
        // Инициализируем бесконечными значениями расстояния для каждого канала (R, G, B)
        let mut min_dist_r = f32::INFINITY;
        let mut min_dist_g = f32::INFINITY;
        let mut min_dist_b = f32::INFINITY;
        let mut min_dist_global = f32::INFINITY;

        for line in lines {
            // Формула расстояния от точки p_font до отрезка AB (в Font Units)
            let ab = line.end - line.start;
            let ap = p_font - line.start;
            let t = (ap.dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            let closest_point = line.start + t * ab;
            let dist_font = (p_font - closest_point).length();

            // Переводим полученное геометрическое расстояние в пиксели текстуры
            let dist_tex = dist_font * scale;

            if dist_tex < min_dist_global {
                min_dist_global = dist_tex;
            }

            // Раскладываем по каналам на основе битовой маски цвета линии
            if (line.color & 0b001) != 0 {
                // Содержит Красный (Red)
                if dist_tex < min_dist_r {
                    min_dist_r = dist_tex;
                }
            }
            if (line.color & 0b010) != 0 {
                // Содержит Зеленый (Green)
                if dist_tex < min_dist_g {
                    min_dist_g = dist_tex;
                }
            }
            if (line.color & 0b100) != 0 {
                // Содержит Синий (Blue)
                if dist_tex < min_dist_b {
                    min_dist_b = dist_tex;
                }
            }
        }

        // Если для какого-то канала не нашлось линий его цвета поблизости,
        // дублируем туда глобальное минимальное расстояние, чтобы избежать швов
        if min_dist_r == f32::INFINITY {
            min_dist_r = min_dist_global;
        }
        if min_dist_g == f32::INFINITY {
            min_dist_g = min_dist_global;
        }
        if min_dist_b == f32::INFINITY {
            min_dist_b = min_dist_global;
        }

        // --- ПОДЗАДАЧА В: Присвоение знака и нормализация ---
        // Если точка снаружи, знак минус. Если внутри — плюс.
        let sign = if is_inside { 1.0 } else { -1.0 };

        let signed_dist_r = min_dist_r * sign;
        let signed_dist_g = min_dist_g * sign;
        let signed_dist_b = min_dist_b * sign;

        // Кодируем float-расстояния в байты u8 (0..255)
        let r_byte = (((signed_dist_r.clamp(-px_range, px_range) + px_range) / (2.0 * px_range))
            * 255.0)
            .round() as u8;
        let g_byte = (((signed_dist_g.clamp(-px_range, px_range) + px_range) / (2.0 * px_range))
            * 255.0)
            .round() as u8;
        let b_byte = (((signed_dist_b.clamp(-px_range, px_range) + px_range) / (2.0 * px_range))
            * 255.0)
            .round() as u8;

        // Записываем данные в текущий пиксель картинки
        *pixel = Rgb([r_byte, g_byte, b_byte]);
    }

    img_buffer
}

pub fn load() {
    let mut file =
        File::open("assets/fonts/Arimo-Bold.ttf").expect("Не удалось открыть файл шрифта");
    let mut font_data = Vec::new();
    file.read_to_end(&mut font_data).expect("Ошибка чтения");

    // 3. Инициализируем парсер (индекс 0 нужен для коллекций шрифтов .ttc)
    let face = Face::parse(&font_data, 0).expect("Ошибка парсинга структуры TTF");

    // 4. Переводим символ в системный ID глифа (Glyph ID)
    let character = 'A';
    let glyph_id = face
        .glyph_index(character)
        .expect("Символ не найден в данном шрифте");

    println!("Анализ символа '{}' (Glyph ID: {:?}):", character, glyph_id);

    let mut builder = MyGeometryBuilder {
        glyph: GlyphGeometry {
            contours: Vec::new(),
        },
        tolerance: 1.0,
        current_start: glam::Vec2::ZERO,
        current_pos: glam::Vec2::ZERO,
    };

    // 2. Запускаем парсинг контура.
    // ttf-parser сам вызовет все нужные методы внутри builder.
    let _bounding_box = face
        .outline_glyph(glyph_id, &mut builder)
        .expect("У этого глифа нет векторного контура (например, это пробел)");

    // 3. Забираем готовую, отсортированную геометрию из билдера
    let glyph_geometry = builder.glyph;

    // Теперь у вас в руках объект glyph_geometry, с которым можно делать Задачу 1!
    println!(
        "Успешно распарсено контуров: {}",
        glyph_geometry.contours.len()
    );
    for (i, contour) in glyph_geometry.contours.iter().enumerate() {
        println!(
            "  Контур {}: содержит {} точек после линеаризации",
            i,
            contour.points.len()
        );
    }

    // 4. Получаем метрики для сдвига каретки текста
    let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0);
    println!("Ширина хода каретки (Advance): {}", advance);

    let mut line_list: Vec<Line> = Vec::new();
    for contour in glyph_geometry.contours {
        if contour.points.len() < 2 {
            continue;
        }

        // Шаг 1: Собираем ребра контура (как у вас, но компактнее через .windows())
        let mut lines: Vec<Line> = Vec::new();
        for pts in contour.points.windows(2) {
            lines.push(Line {
                start: pts[0],
                end: pts[1],
                color: 0,
            });
        }
        // Замыкаем контур
        lines.push(Line {
            start: *contour.points.last().unwrap(),
            end: contour.points[0],
            color: 0,
        });

        let n = lines.len();
        if n == 0 {
            continue;
        }

        // Шаг 2: Ищем острые углы между соседними ребрами
        // Будем хранить флаг true, если между резанными линиями i и i+1 есть острый угол
        let mut is_corner = vec![false; n];

        // Задаем порог: угол считается острым, если изменение направления > ~35 градусов
        // Косинус 35 градусов примерно 0.81. Чем меньше число, тем острее должен быть угол.
        let corner_threshold = 0.81;

        for i in 0..n {
            let next_i = (i + 1) % n;

            let dir1 = (lines[i].end - lines[i].start).normalize_or_zero();
            let dir2 = (lines[next_i].end - lines[next_i].start).normalize_or_zero();

            // Скалярное произведение нормализованных векторов — это чистый косинус угла
            let cos_angle = dir1.dot(dir2);

            // Если косинус меньше порога, значит линии резко разошлись (есть угол)
            if cos_angle < corner_threshold {
                is_corner[i] = true;
            }
        }

        // Шаг 3: Раскраска ребер
        let mut color_index = 0;

        // Красим первое ребро стартовым цветом
        lines[0].color = COLORS[color_index];

        // Проходим по всем остальным ребрам
        for i in 1..n {
            // Если перед этим ребром (в точке соединения с предыдущим) был острый угол,
            // переключаем цвет на следующий по кругу
            if is_corner[i - 1] {
                color_index = (color_index + 1) % 3;
            }
            lines[i].color = COLORS[color_index];
        }

        // Важная проверка стыка конца и начала:
        // Если последнее ребро и первое ребро имеют одинаковый цвет, но между ними ЕСТЬ угол,
        // нужно перекрасить последнее ребро, чтобы не ломать логику шейдера
        if is_corner[n - 1] && lines[n - 1].color == lines[0].color {
            lines[n - 1].color = COLORS[(color_index + 1) % 3];
        }
        line_list.append(&mut lines);
    }

    let img = generate_msdf_image(&line_list, 64, 64, face.units_per_em() as f32, 48.0);

    img.save("./aboba.jpg").unwrap();
}
