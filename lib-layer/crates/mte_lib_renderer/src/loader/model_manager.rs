// use std::fs::File;
// use std::io::{BufWriter, Write};

// // Структура вершины (32 байта)
// #[derive(Clone, Copy, Debug)]
// #[repr(C)]
// struct Vertex {
//     position: [f32; 3],
//     normal: [f32; 3],
//     tex_coords: [f32; 2],
// }

// // Бинарная структура метаданных (32 байта, фиксированный размер)
// // Больше никаких строк и динамических массивов для мгновенного чтения с диска
// #[derive(Clone, Copy, Debug)]
// #[repr(C)]
// struct BinaryPrimitiveInfo {
//     mesh_id: u32,             // Числовой ID меша вместо строки
//     primitive_index: u32,     // Индекс примитива внутри этого меша
//     vertex_offset_bytes: u64, // Смещение блока вершин в файле геометрии
//     vertex_count: u64,        // Количество вершин
//     index_offset_bytes: u64,  // Смещение блока индексов в файле геометрии
//     index_count: u64,         // Количество индексов (0, если их нет)
// }

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let mut data_file = BufWriter::new(File::create("geometry_data.bin")?);
//     let mut meta_file = BufWriter::new(File::create("geometry_meta.bin")?);

//     let (document, buffers, _) = gltf::import("model.gltf")?;

//     let mut primitives_meta = Vec::new();
//     let mut current_bin_offset: u64 = 0;

//     // Обходим меши. Используем enumerate(), чтобы получить числовой mesh_id
//     for (mesh_id, mesh) in document.meshes().enumerate() {
//         for (p_idx, primitive) in mesh.primitives().enumerate() {
//             let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|b| &b.0[..]));

//             // Извлекаем геометрию
//             let positions = reader.read_positions().ok_or("Missing positions")?;
//             let mut normals = reader.read_normals();
//             let mut tex_coords = reader.read_tex_coords(0).map(|tc| tc.into_f32());

//             let vertex_count = positions.len();
//             let mut primitive_vertices = Vec::with_capacity(vertex_count);
//             let mut pos_iter = positions;

//             for _ in 0..vertex_count {
//                 let pos = pos_iter.next().unwrap();
//                 let norm = normals.as_mut().and_then(|n| n.next()).unwrap_or([0.0, 0.0, 0.0]);
//                 let uv = tex_coords.as_mut().and_then(|t| t.next()).unwrap_or([0.0, 0.0]);

//                 primitive_vertices.push(Vertex {
//                     position: pos,
//                     normal: norm,
//                     tex_coords: uv,
//                 });
//             }

//             let mut primitive_indices = Vec::new();
//             if let Some(indices_reader) = reader.read_indices() {
//                 primitive_indices = indices_reader.into_u32().collect::<Vec<u32>>();
//             }

//             // Вычисляем размеры в байтах
//             let v_bytes_size = (primitive_vertices.len() * std::mem::size_of::<Vertex>()) as u64;
//             let i_bytes_size = (primitive_indices.len() * std::mem::size_of::<u32>()) as u64;

//             let vertex_offset = current_bin_offset;
//             let index_offset = vertex_offset + v_bytes_size;

//             // Записываем вершины в geometry_data.bin
//             let v_slice = unsafe {
//                 std::slice::from_raw_parts(primitive_vertices.as_ptr() as *const u8, v_bytes_size as usize)
//             };
//             data_file.write_all(v_slice)?;

//             // Записываем индексы в geometry_data.bin
//             if !primitive_indices.is_empty() {
//                 let i_slice = unsafe {
//                     std::slice::from_raw_parts(primitive_indices.as_ptr() as *const u8, i_bytes_size as usize)
//                 };
//                 data_file.write_all(i_slice)?;
//             }

//             // Добавляем бинарную структуру метаданных в массив
//             primitives_meta.push(BinaryPrimitiveInfo {
//                 mesh_id: mesh_id as u32,
//                 primitive_index: p_idx as u32,
//                 vertex_offset_bytes: vertex_offset,
//                 vertex_count: vertex_count as u64,
//                 index_offset_bytes: index_offset,
//                 index_count: primitive_indices.len() as u64,
//             });

//             current_bin_offset += v_bytes_size + i_bytes_size;
//         }
//     }

//     // --- Запись файла метаданных ---
//     // 1. Сначала пишем u64 — сколько всего примитивов записано в файле
//     let total_primitives = primitives_meta.len() as u64;
//     meta_file.write_all(&total_primitives.to_ne_bytes())?;

//     // 2. Затем пишем весь массив структур целиком как сырые байты
//     let meta_bytes_size = primitives_meta.len() * std::mem::size_of::<BinaryPrimitiveInfo>();
//     let meta_slice = unsafe {
//         std::slice::from_raw_parts(primitives_meta.as_ptr() as *const u8, meta_bytes_size)
//     };
//     meta_file.write_all(meta_slice)?;

//     println!("Экспорт завершен. Сгенерировано два бинарных файла.");
//     Ok(())
// }

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub type ModelIdType = usize;
pub type MdlFileIdType = u32;

pub struct ModelManager {
    models_files: Vec<MdlFileIdType>,
    assets_path: PathBuf,
}

impl ModelManager {
    pub fn create(assets_path: PathBuf) {
        let mut registry_path = assets_path.clone();
        registry_path.push("registry.bin");
        if !registry_path.exists() {}
    }

    pub fn create_registry() {}

    pub fn load_from_registry(assets_path: PathBuf) -> anyhow::Result<Self> {
        let mut registry_path = assets_path.clone();
        registry_path.push("registry.bin");
        let file = File::open(registry_path)?;

        let reader = BufReader::new(file);

        let data: Vec<MdlFileIdType> = bincode::deserialize_from(reader)?;

        Ok(Self {
            models_files: data,
            assets_path,
        })
    }
}
