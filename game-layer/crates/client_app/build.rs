// use std::env;
// use std::path::PathBuf;

pub fn main() {}
//     println!("cargo::rerun-if-changed=build.rs");
//     // println!("cargo::rerun-if-changed=assets");

//     // let assets_folder = find_assets().expect("Не обнаружена папка assets");

//     // --config "Путь"  Если аргумент указан, то используем указанный конфиг, иначе генерируем свой и используем путь к нему.
//     // В конфиге лежит всё, что нужно конфигурировать.
//     // Стандартные пути вшиваются

//     let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Не найдена папка крейта");
//     let mut manifest_dir = PathBuf::from(manifest_dir);

//     let build_config = manifest_dir.join("build.toml");

//     // let mut builder = VfsBuilder::new(&assets_folder);

//     // let pak_name = "global";
//     // let mut pak_path = assets_folder.clone();
//     // pak_path.push(pak_name);
//     // let mut pak1 = PakBuilder::create_pak(&pak_path.with_extension("pak"));

//     // // pak1.add_dir(&assets_folder);
//     // pak1.add(format!(
//     //     "{}/{}",
//     //     assets_folder.to_str().unwrap(),
//     //     "fasn-kukuruza.jpg"
//     // ));
//     // pak1.add(format!(
//     //     "{}/{}",
//     //     assets_folder.to_str().unwrap(),
//     //     "hdr.wgsl"
//     // ));

//     // builder.add_pak(pak1);

//     // builder.add_dir(&assets_folder);

//     // builder.assemble().unwrap();
// }
