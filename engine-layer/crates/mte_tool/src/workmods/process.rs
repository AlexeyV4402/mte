use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{array, fs};

use lib_core::fs::os::get_crate_dir_from;
use lib_core::fs::vfs::builder::init_builder_state;
use lib_core::fs::vfs::config::CompressionType;
use lib_core::fs::vfs::resolve::{get_cache_path, primary_pack_resolve, resolve_path_checked};
use lib_core::fs::vfs::types::PostBakeVfsEntry;
use mte_macros::vpath;
use naga::back::spv;
use naga::front::glsl;
use naga::{FastHashMap, valid};

use crate::process_assets::images::process_png_jpg;
use crate::process_assets::shaders;
use crate::rs_parser::syn_parser::run_syn_parser;
use crate::types::{FileFormatDefine, PreResolveAssetData};
use crate::utils::{detect_file_format_by_signature, get_rs_files};

const VALID_SHADER_PURPOSES: &[&str] = &["vertex", "fragment", "compute"];
const TEXT_FORMATS: [&'static str; 1] = ["glsl"];

fn process_asset(os_path: &Path, compression: &CompressionType) -> Result<Vec<u8>, String> {
    let input_content = fs::read(os_path).map_err(|e| format!("Ошибка чтения файла: {}", e))?;

    let file_name = os_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Невалидное имя файла".to_string())?;

    let parts: Vec<&str> = file_name.split('.').collect();

    let declared_ext = os_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| format!("Файл {:?} не имеет расширения", os_path))?;

    if !TEXT_FORMATS.contains(&declared_ext.as_str()) {
        if input_content.len() >= 16 {
            if let Some(real_kind) = detect_file_format_by_signature(&input_content[0..16]) {
                let real_ext = real_kind.extension();
                if real_ext != declared_ext && !(real_ext == "jpg" && declared_ext == "jpeg") {
                    return Err(format!(
                        "Сигнатурный сбой: файл {:?} притворяется '{}', но сигнатура говорит '{}'!",
                        os_path, declared_ext, real_ext
                    ));
                }
            }
        }
    }

    let purpose = if parts.len() >= 3 {
        Some(parts[parts.len() - 2].to_lowercase())
    } else {
        None
    };

    let output_content = match compression {
        CompressionType::None => input_content,

        CompressionType::Unpack | CompressionType::Auto => match declared_ext.as_str() {
            "png" | "jpg" | "jpeg" => {
                let img = image::load_from_memory(&input_content)
                    .map_err(|e| format!("Ошибка декодирования картинки: {}", e))?;
                img.to_rgba8().into_raw()
            }

            "glsl" => {
                if let Some(ref p) = purpose {
                    if VALID_SHADER_PURPOSES.contains(&p.as_str()) {
                        shaders::process_glsl(input_content, p)?
                    } else {
                        return Err("Неизвестное назначение".to_string());
                    }
                } else {
                    input_content
                }
            }

            _ => input_content,
        },
    };

    Ok(output_content)
}

pub fn process(args: &[String], release: bool) -> Result<(), String> {
    let mut file_option: Option<PathBuf> = None;

    for argument in args.iter().skip(2) {
        if argument == "--release" {
        } else if let Some(val) = argument.strip_prefix("--file=") {
            if !val.is_empty() {
                file_option = Some(PathBuf::from(val));
            }
        }
    }

    let workspace_root = vpath!("workspace://");

    let scan_path = file_option.as_deref().unwrap_or(&workspace_root);

    let files_to_scan = get_rs_files(scan_path);

    let out = run_syn_parser(files_to_scan);

    let mut parser_type_groups: [HashMap<PathBuf, Vec<PreResolveAssetData>>; 2] =
        array::from_fn(|idx| HashMap::new());

    for asset in out {
        parser_type_groups[asset.resolver_type as usize]
            .entry(asset.caller)
            .or_default()
            .push(PreResolveAssetData::new(asset.vpath, asset.format_define));
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
    parser_type_groups[0].iter().for_each(|(caller, assets_data)| {
        // Собираем вектор виртуальных путей. 
        // Понадобится для определения текущего обрабатываемого ассета.
        let vpaths: Vec<String> = assets_data.iter().map(|asset_data| asset_data.vpath.clone()).collect();

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
                    .for_each(|(result, asset_data)| {
                        print!("Обработка ассета {}; ", asset_data.vpath);
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
                                            vpath_str: asset_data.vpath.clone(),
                                            pack_path: pack_path,
                                            offset: current_offset,
                                            length: cache.len() as u64,
                                        });

                                        counter += 1;
                                        println!("Ассет успешно обработан;");
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

    parser_type_groups[1].iter().for_each(|(caller, assets_data)| {
        assets_data.iter().for_each(|asset_data| {
            print!("Обработка ассета {}; ", asset_data.vpath);
            match resolve_path_checked(&asset_data.vpath, &workspace_root, Some(caller.to_path_buf())) {
                Ok(os_path) => {
                    match process_asset(&os_path, &CompressionType::Auto) {
                        Ok(cache) => {
                            let cache_path = match get_cache_path(&os_path) {
                                Ok(cache_path) => cache_path,
                                Err(err) => {println!("Пропуск ассета (не удалось определить путь кэша): {}", err); return;}
                            };
                            match fs::write(&cache_path, &cache) {
                                Ok(_) => {},
                                Err(err) => {
                                    println!("Пропуск ассета (ошибка записи кэша): {}", err);
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

                            let crate_vfs = vfs_metas.entry(crate_dir.clone()).or_default();

                            crate_vfs.push(PostBakeVfsEntry {
                                vpath_hash: counter,
                                vpath_str: asset_data.vpath.clone(),
                                pack_path: cache_path,
                                offset: 0,
                                length: cache.len() as u64,
                            });

                            counter += 1;
                            println!("Ассет успешно обработан;");
                        },
                        Err(err) => {
                            println!("Пропуск ассета (ошибка обработки): {}", err)
                        }
                    }
                },
                Err(err) => println!("Пропуск ассета (ошибка резолва): {}", err),
            };

        });
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
