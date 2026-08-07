pub struct MonolithAsset {
    pub vertices: Vec<f32>, // Вообще ВСЕ координаты [x, y, z, x, y, z...] из файла
    pub indices: Vec<u32>,  // Вообще ВСЕ индексы, склеенные в один поток
}

pub fn load_glb_as_monolith(glb_bytes: &[u8]) -> Result<MonolithAsset, String> {
    // 1. Распаковываем GLB в память
    let (document, buffers, _) =
        gltf::import_slice(glb_bytes).map_err(|e| format!("Не удалось распарсить GLB: {}", e))?;

    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut global_vertex_offset = 0u32; // Смещение, чтобы треугольники не перепутались

    // 2. Тупо перебираем вообще всё, что есть в файле
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            // Читаем индексы текущего кусочка
            let mut prim_indices = Vec::new();
            if let Some(indices_iterator) = reader.read_indices() {
                prim_indices = indices_iterator.into_u32().collect();
            }

            // Читаем вершины текущего кусочка
            let mut prim_vertices = Vec::new();
            if let Some(positions_iterator) = reader.read_positions() {
                for pos in positions_iterator {
                    prim_vertices.push(pos[0]); // x
                    prim_vertices.push(pos[1]); // y
                    prim_vertices.push(pos[2]); // z
                }
            }

            // Накатываем глобальное смещение на индексы
            for idx in prim_indices {
                all_indices.push(idx + global_vertex_offset);
            }

            // Сдвигаем смещение на количество добавленных вершин
            global_vertex_offset += (prim_vertices.len() / 3) as u32;

            // Сваливаем все вершины в общий котел
            all_vertices.extend(prim_vertices);
        }
    }

    Ok(MonolithAsset {
        vertices: all_vertices,
        indices: all_indices,
    })
}

use meshopt::{
    generate_vertex_remap, optimize_vertex_fetch, remap_index_buffer, remap_vertex_buffer
};

pub struct OptimizedAsset {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
}

use meshopt::ffi::{
    meshopt_generateVertexRemap, meshopt_optimizeVertexFetch, meshopt_remapIndexBuffer, meshopt_remapVertexBuffer
};

pub fn optimize_monolith(asset: &MonolithAsset) -> OptimizedAsset {
    if asset.vertices.is_empty() || asset.indices.is_empty() {
        return OptimizedAsset {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
    }

    // 1. Считаем правильные размеры
    let source_vertex_count = asset.vertices.len() / 3; // Сколько всего вершин у нас на входе
    let index_count = asset.indices.len(); // Сколько всего индексов у нас на входе
    let vertex_stride = std::mem::size_of::<f32>() * 3; // Шаг вершины (12 байт для x,y,z)

    // 2. Создаем временную таблицу remap нужного размера.
    // По спецификации meshopt, её размер должен быть равен количеству ИСХОДНЫХ вершин.
    let mut remap: Vec<u32> = vec![0; source_vertex_count];

    // 3. Вызываем генерацию ремапа напрямую через FFI
    let unique_vertex_count = unsafe {
        meshopt_generateVertexRemap(
            remap.as_mut_ptr(),
            asset.indices.as_ptr(),
            index_count,
            asset.vertices.as_ptr() as *const std::ffi::c_void,
            source_vertex_count,
            vertex_stride,
        )
    };

    // 4. Выделяем память под новые ОПТИМИЗИРОВАННЫЕ буферы
    // Индексов останется столько же, а вот вершин станет МЕНЬШЕ (уйдут дубликаты)
    let mut remapped_indices: Vec<u32> = vec![0; index_count];
    let mut remapped_vertices: Vec<f32> = vec![0.0; unique_vertex_count * 3];

    // 5. Пересобираем буферы на основе сгенерированной таблицы remap
    unsafe {
        meshopt_remapIndexBuffer(
            remapped_indices.as_mut_ptr(),
            asset.indices.as_ptr(),
            index_count,
            remap.as_ptr(),
        );

        meshopt_remapVertexBuffer(
            remapped_vertices.as_mut_ptr() as *mut std::ffi::c_void,
            asset.vertices.as_ptr() as *const std::ffi::c_void,
            source_vertex_count,
            vertex_stride,
            remap.as_ptr(),
        );

        // 6.Vertex Fetch Optimization (Упорядочиваем вершины для DirectStorage)
        meshopt_optimizeVertexFetch(
            remapped_vertices.as_mut_ptr() as *mut std::ffi::c_void,
            remapped_indices.as_mut_ptr(),
            index_count,
            remapped_vertices.as_ptr() as *const std::ffi::c_void,
            unique_vertex_count,
            vertex_stride,
        );
    }

    OptimizedAsset {
        vertices: remapped_vertices,
        indices: remapped_indices,
    }
}

use std::io::Cursor;

use gltf::Gltf;
use image::{GenericImageView, ImageReader};

use crate::process_assets::images::process_png_jpg;

// Наша структура вершины перед отправкой в meshopt и пак
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PackedVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

pub fn process_glb_to_pack(glb_bytes: &[u8]) -> Result<(), String> {
    // 1. Импортируем GLB пирог. gltf автоматически разделяет JSON-структуру и бинарный буфер (blob)
    let gltf = Gltf::from_slice(glb_bytes).map_err(|e| format!("Ошибка парсинга GLB: {}", e))?;
    let blob = gltf
        .blob
        .as_ref()
        .ok_or("В GLB отсутствует бинарный blob с данными")?;

    // Массив для хранения обработанных текстур (чтобы сопоставлять их по индексу материала)
    let mut processed_textures_scale = Vec::new();

    // ==========================================
    // ШАГ 1: ИЗВЛЕЧЕНИЕ И СЖАТИЕ ТЕКСТУР (ПЕРВЫМ ДЕЛОМ)
    // ==========================================
    for texture in gltf.textures() {
        let source = texture.source();

        let (width_orig, height_orig, raw_rgba_bytes) = match source.source() {
            gltf::image::Source::View { view, .. } => {
                // Извлекаем сжатые байты PNG/JPG прямо из общего blob по смещению
                let start = view.offset();
                let end = start + view.length();
                let compressed_bytes = &blob[start..end];

                // Декодируем в сырые пиксели
                let img = ImageReader::new(Cursor::new(compressed_bytes))
                    .with_guessed_format()
                    .map_err(|e| e.to_string())?
                    .decode()
                    .map_err(|e| e.to_string())?;

                let (w, h) = img.dimensions();
                (w, h, img.to_rgba8().into_raw())
            }
            _ => return Err("Текстура должна быть зашита внутрь GLB (Source::View)".to_string()),
        };

        // Передаем сырые байты в вашу функцию process_png_jpg, которую мы писали ранее.
        // На выходе получаем KTX2/DDS с BC7 сжатием.
        let ktx2_bytes = process_png_jpg(&raw_rgba_bytes)?;

        // !!! КРИТИЧЕСКИ ВАЖНО !!!
        // Узнаем, до каких размеров ctt молча расширил картинку (например, 498 -> 500)
        // Для этого читаем заголовок ktx2_bytes или берем алгоритм паддинга:
        let width_gpu = width_orig + (4 - (width_orig % 4)) % 4;
        let height_gpu = height_orig + (4 - (height_orig % 4)) % 4;

        // Запоминаем коэффициенты масштабирования для UV этой текстуры
        let u_scale = width_orig as f32 / width_gpu as f32;
        let v_scale = height_orig as f32 / height_gpu as f32;

        processed_textures_scale.push((u_scale, v_scale));

        // TODO: Записать ktx2_bytes в ваш финальный пак и сохранить смещение!
    }

    // ==========================================
    // ШАГ 2: РАЗБОР МЕШЕЙ НА ОТДЕЛЬНЫЕ ПРИМИТИВЫ
    // ==========================================
    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            // Читаем индексы примитива
            let reader = primitive.reader(|buffer| Some(blob));

            let mut indices: Vec<u32> = if let Some(indices_reader) = reader.read_indices() {
                indices_reader.into_u32().collect()
            } else {
                return Err("Примитив должен иметь индексный буфер!".to_string());
            };

            // Читаем сырые вершины
            let positions = reader.read_positions().ok_or("Нет позиций")?;
            let mut tex_coords: Vec<[f32; 2]> = if let Some(uv_reader) = reader.read_tex_coords(0) {
                uv_reader.into_f32().collect()
            } else {
                // Если у меша нет UV, зануляем
                vec![[0.0, 0.0]; positions.len()]
            };
            let normals = reader.read_normals().ok_or("Нет нормалей")?;

            // ==========================================
            // ШАГ 3: КОРРЕКЦИЯ UV-КООРДИНАТ (ФИКС СЪЕЗЖАНИЯ)
            // ==========================================
            let material = primitive.material();

            // А) Компенсация паддинга BC7-сжатия
            if let Some(texture_info) = material.pbr_metallic_roughness().base_color_texture() {
                let tex_index = texture_info.texture().index();
                let (u_scale, v_scale) = processed_textures_scale[tex_index];

                // Б) Проверяем наличие мета-матрицы KHR_texture_transform
                // Некоторые экспортеры (Blender) ужимают UV и пишут сдвиг/масштаб сюда
                let mut offset = [0.0, 0.0];
                let mut scale = [1.0, 1.0];
                if let Some(transform) = texture_info.texture_transform() {
                    offset = transform.offset();
                    scale = transform.scale();
                    // Примечание: если есть вращение transform.rotation(), его тоже нужно применить через синус/косинус
                }

                // Применяем трансформацию к каждой вершине примитива по цепочке
                for uv in tex_coords.iter_mut() {
                    // Сначала применяем внутренний сдвиг GLTF модели
                    uv[0] = uv[0] * scale[0] + offset[0];
                    uv[1] = uv[1] * scale[1] + offset[1];

                    // Затем умножаем на коэффициент паддинга BC7, чтобы текстура не сплющилась!
                    uv[0] *= u_scale;
                    uv[1] *= v_scale;
                }
            }

            // Сшиваем данные в нашу структуру PackedVertex
            let vertices: Vec<PackedVertex> = positions
                .zip(tex_coords)
                .zip(normals)
                .map(|((p, uv), n)| PackedVertex {
                    position: p,
                    tex_coords: uv,
                    normal: n,
                })
                .collect();

            // ==========================================
            // ШАГ 4: СКАРМЛИВАЕМ ПРИМИТИВ В MESHOPT
            // ==========================================
            // 1. Оптимизируем кэш вершин (Vertex Cache Optimization)
            meshopt::optimize_vertex_cache_in_place(&mut indices, vertices.len());

            // 2. Убираем дубликаты и оптимизируем овердроу (Overdraw)
            // meshoptimizer::optimize_overdraw_in_place(...);

            // 3. Переупорядочиваем буфер вершин в соответствии с оптимизированными индексами
            let optimized_vertices: Vec<PackedVertex> =
                meshopt::optimize_vertex_fetch(&mut indices, &vertices);

            // ==========================================
            // ШАГ 5: ЗАПИСЬ ОПТИМИЗИРОВАННОГО ПРИМИТИВА В ПАК
            // ==========================================
            // В этот момент optimized_vertices и indices — это идеальные,
            // аппаратно-сжатые и выровненные блоки данных.
            // Записываем их в geometry.pack, а в Манифест пишем:
            // "Primitive_X: vertex_offset, vertex_count, index_offset, index_count"
        }
    }

    Ok(())
}
