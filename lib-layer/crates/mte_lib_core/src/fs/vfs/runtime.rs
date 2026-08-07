// use std::collections::HashMap;
// use std::fs::File;
// use std::path::{Path, PathBuf};
// use std::sync::OnceLock;

// use memmap2::Mmap;

// // use crate::fs::vfs::builder::VfsEntry;

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum MmapPolicy {
//     None,
//     AllPacked,
//     PackedLargerThan(u64),
// }

// #[derive(Default)]
// pub struct VfsRuntime {
//     pub open_files: HashMap<u64, File>,
//     pub mmaps: HashMap<u64, Mmap>,
//     pub data_dir_path: PathBuf,
// }

// pub static VFS: OnceLock<VfsRuntime> = OnceLock::new();

// pub fn vfs_runtime_auto<P: AsRef<Path>>(
//     meta_path: P,
//     mmap_policy: MmapPolicy,
// ) -> std::io::Result<()> {
//     // 1. Читаем бинарный vfs.meta
//     let meta_bytes = std::fs::read(meta_path)?;
//     let meta_content: Vec<VfsEntry> = bincode::deserialize(&meta_bytes)
//         .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

//     let mut open_files = HashMap::new();
//     let mut mmaps = HashMap::new();

//     // 2. Итерируемся по записям и декларативно распределяем ресурсы
//     for entry in &meta_content {
//         if !open_files.contains_key(&entry.vpath_hash) {
//             let file = File::open(&entry.phys_path)?;
//             let need_mmap = match mmap_policy {
//                 MmapPolicy::None => false,
//                 MmapPolicy::AllPacked => entry.storage_type == StorageType::Packed,
//                 MmapPolicy::PackedLargerThan(limit) => {
//                     entry.storage_type == StorageType::Packed && entry.length >= limit
//                 }
//             };

//             if need_mmap {
//                 let mmap = unsafe { Mmap::map(&file)? };
//                 mmaps.insert(entry.vpath_hash, mmap);
//             }

//             open_files.insert(entry.vpath_hash, file);
//         }
//     }

//     let runtime = VfsRuntime {
//         open_files,
//         mmaps,
//         // Поле data_dir_path пока оставляем пустым или дефолтным, раз мы его отложили
//         data_dir_path: PathBuf::new(),
//     };

//     VFS.set(runtime)
//         .ok()
//         .expect("Ошибка: VFS уже была инициализирована!");
//     Ok(())
// }
