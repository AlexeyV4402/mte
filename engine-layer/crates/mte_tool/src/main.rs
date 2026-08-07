use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use lib_core::as_u8_slice;
use lib_core::fs::os::get_crate_dir_from;
use lib_core::fs::vfs::builder::init_builder_state;
use lib_core::fs::vfs::config::CompressionType;
use lib_core::fs::vfs::resolve::{primary_pack_resolve, resolve_path_checked};
use lib_core::fs::vfs::types::PostBakeVfsEntry;
use mte_macros::vpath;

use crate::process_assets::images::process_png_jpg;
use crate::process_assets::models::{load_glb_as_monolith, optimize_monolith, process_glb_to_pack};
use crate::rs_parser::{run_regex_parser, run_syn_parser};
use crate::utils::{detect_file_format, get_rs_files};

mod process_assets;
mod rs_parser;
mod utils;

fn process_asset(os_path: &Path, compression: &CompressionType) -> Result<Vec<u8>, String> {
    let input_content = fs::read(os_path).map_err(|e| format!("Ошибка чтения файла: {}", e))?;

    if input_content.len() < 16 {
        return Err("Файл слишком короткий. Невозможно определить сигнатуру".to_string());
    }

    let output_content = match compression {
        CompressionType::None => input_content,
        CompressionType::Auto => {
            if let Some(kind) = detect_file_format(&input_content[0..=16]) {
                let ext = kind.extension();
                match ext {
                    "jpg" | "png" => process_png_jpg(&input_content)?,
                    "glb" => {
                        let output = process_glb_to_pack(&input_content);
                        Vec::from(as_u8_slice!(&optimized_data))
                    }
                    unknown => return Err(format!("Неизвестный формат файла: {}", unknown)),
                }
            } else {
                return Err("Формат файла неизвестен".to_string());
            }
        }
    };
    let cache_name = os_path.to_string_lossy().replace("/", "\\");
    let cache_folder = vpath!("workspace://").join(".mte_cache").join(cache_name);
    fs::write(&cache_folder, &output_content)
        .map_err(|err| format!("Ошибка записи файла: {}", err))?;
    Ok(output_content)
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    let mut release = false;
    let mut file_option: Option<PathBuf> = None;

    // Изящный парсинг аргументов
    for argument in args.iter().skip(1) {
        // Пропускаем имя самого бинарника
        if argument == "--release" {
            release = true;
        } else if let Some(val) = argument.strip_prefix("--file=") {
            if !val.is_empty() {
                file_option = Some(PathBuf::from(val));
            }
        }
    }

    let workspace_root = vpath!("workspace://");

    let scan_path = file_option.as_deref().unwrap_or(&workspace_root);

    let files_to_scan = get_rs_files(scan_path);

    let out = if release {
        run_syn_parser(files_to_scan)
    } else {
        run_regex_parser(files_to_scan)
    };

    // Словарь <Вызвавший файл, Вектор пар (Имя ассета, Виртуальный путь)>. Для оптимизации.
    let mut grouped_files: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();

    // Сборка словаря.
    for (asset_name, caller_path) in out {
        grouped_files
            .entry(caller_path)
            .or_default()
            .push((asset_name.clone(), asset_name));
    }

    // Инициализация структуры для переиспользования данных.
    init_builder_state(&workspace_root)
        .map_err(|err| format!("Ошибка инициализации состояния сборщика: {}", err))?;

    // Хэшмап, где ключ - физический к папке крейта.
    let mut vfs_metas: HashMap<PathBuf, Vec<PostBakeVfsEntry>> = HashMap::new();

    // Хэшмап, где ключ - физический путь к паку.
    let mut pack_files: HashMap<PathBuf, File> = HashMap::new();
    let mut counter = 0;

    // Основной цикл обработки. Идём итератором по словарю.
    grouped_files.iter().for_each(|(caller, assets_data)| {
        // Собираем вектор виртуальных путей. 
        // Понадобится для определения текущего обрабатываемого ассета.
        let vpaths: Vec<String> = assets_data.iter().map(|(_, vpath)| vpath.clone()).collect();

        // 'primary_pack_resolve' решает пути для каждого ассета в векторе, сопоставленному вызывающему файлу. Возвращает Result<Vec<...>, _>.
        match primary_pack_resolve(&vpaths, &workspace_root, Some(caller.to_path_buf())) {
            Ok(path_vec) => {

                // Поиск папки для контента, для каждого вызывающего файла. В неё пихаются паки.
                let content_dir = match resolve_path_checked("content://", &workspace_root, Some(caller.to_path_buf())) {
                    Ok(content_dir) => {
                        content_dir
                    },
                    Err(err) => {
                        println!("Пропуск вызывающего файла {};\nОшибка нахождения папки для контента: {};\nУбедитесь, что она указана в vfs.toml", caller.display(), err);
                        return;
                    },
                };

                let crate_dir = match get_crate_dir_from(caller) {
                    Ok(crate_dir) => {
                        crate_dir
                    },
                    Err(err) => {
                        println!("Пропуск вызывающего файла {};\nОшибка нахождения корная крейта: {};", caller.display(), err);
                        return;
                    },
                };

                // Итерация по каждому решённому ассету.
                path_vec
                    .iter()
                    .zip(assets_data.iter())
                    .for_each(|(result, (asset_name, _))| {
                        print!("Обработка ассета {}; ", asset_name);
                        match result {
                            Ok((os_path, pack_name, compression)) => {
                                match process_asset(os_path, compression) {
                                    Ok(cache) => {
                                        let pack_path = content_dir.join(&pack_name);

                                        let pack_file = pack_files.entry(pack_path.clone()).or_insert_with(|| {
                                            OpenOptions::new()
                                                .create(true)
                                                .truncate(true)
                                                .write(true)
                                                .read(true)
                                                .open(&pack_path)
                                                .expect("Ошибка создания файла пака")
                                        });

                                        let current_offset = pack_file.metadata().expect("Ошибка чтения метаданных").len();

                                        pack_file.write_all(&cache).expect("Ошибка записи в пак");

                                        let crate_vfs = vfs_metas.entry(crate_dir.clone()).or_default();

                                        crate_vfs.push(PostBakeVfsEntry {
                                            vpath_hash: counter,
                                            vpath_str: asset_name.clone(),
                                            pack_path: pack_path,
                                            offset: current_offset,
                                            length: cache.len() as u64,
                                        });

                                        counter += 1;
                                    },
                                    Err(err) => {
                                        println!("Пропуск ассета (ошибка обработки): {}", err)
                                    }
                                }
                            }
                            Err(err) => println!("Пропуск ассета (ошибка резолва): {}", err),
                        }
                    });
            }
            Err(err) => {
                println!("Пропуск вызывающего файла {}: {}", caller.display(), err);
            }
        }
    });

    drop(pack_files);

    println!();

    vfs_metas.into_iter().for_each(|(crate_root, content)| {
        print!("Запись .vfs_meta в крейт {}; ", crate_root.display());
        let bytes = match bincode::serialize(&content) {
            Ok(bytes) => bytes,
            Err(err) => {
                println!("Ошибка сериализации: {}", err);
                return;
            }
        };
        match fs::write(crate_root.join(".vfs_meta"), bytes) {
            Ok(_) => println!("Успешно!"),
            Err(err) => println!("Ошибка записи файла: {}", err),
        }
    });

    Ok(())
}
