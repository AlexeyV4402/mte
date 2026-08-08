// #![feature(proc_macro_span_file)]
use std::env;

use lib_core::fs::os::get_workspace_dir_from;
use lib_core::fs::vfs::builder::init_builder_state;
use proc_macro::TokenStream;
use syn::parse::Parser;
use syn::{Expr, Fields, ItemEnum, Lit, LitStr, parse_macro_input};

use crate::find_entry::find_vfs_entry;

mod find_entry;

#[proc_macro]
pub fn vpath(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    let span = proc_macro::Span::call_site();

    let resolve_path = || {
        let workspace_root =
            get_workspace_dir_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
                .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))?;

        init_builder_state(&workspace_root)
            .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))?;

        lib_core::fs::vfs::resolve::resolve_path_checked(
            &input_lit.value(),
            &workspace_root,
            span.local_file(),
        )
        .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))
    };

    match resolve_path() {
        Ok(path) => {
            let components: Vec<&str> = path
                .components()
                .map(|c| c.as_os_str().to_str().unwrap())
                .collect();
            let expanded = quote::quote! {
                {
                    [#(#components),*].iter().collect::<PathBuf>()
                }
            };
            proc_macro::TokenStream::from(expanded)
        }
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}

#[proc_macro]
pub fn vpath_unchecked(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    let span = proc_macro::Span::call_site();

    let resolve_path = || {
        let workspace_root =
            get_workspace_dir_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
                .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))?;

        init_builder_state(&workspace_root)
            .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))?;

        lib_core::fs::vfs::resolve::resolve_path_unchecked(
            &input_lit.value(),
            &workspace_root,
            span.local_file(),
        )
        .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))
    };

    match resolve_path() {
        Ok(path) => {
            let components: Vec<&str> = path
                .components()
                .map(|c| c.as_os_str().to_str().unwrap())
                .collect();
            let expanded = quote::quote! {
                {
                    [#(#components),*].iter().collect::<PathBuf>()
                }
            };
            proc_macro::TokenStream::from(expanded)
        }
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}

#[proc_macro]
pub fn vfs_read(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    let span = proc_macro::Span::call_site();

    let resolve_path = || {
        let workspace_root =
            get_workspace_dir_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
                .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))?;

        lib_core::fs::vfs::resolve::resolve_path_unchecked(
            &input_lit.value(),
            &workspace_root,
            span.local_file(),
        )
        .map_err(|err| syn::Error::new(input_lit.span(), format!("{}", err)))
    };

    match resolve_path() {
        Ok(path) => {
            let components: Vec<&str> = path
                .components()
                .map(|c| c.as_os_str().to_str().unwrap())
                .collect();
            let expanded = quote::quote! {
                {
                    [#(#components),*].iter().collect::<PathBuf>()
                }
            };
            proc_macro::TokenStream::from(expanded)
        }
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}

#[proc_macro]
pub fn vfs_read_all(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    let span = proc_macro::Span::call_site();

    match find_vfs_entry(&input_lit.value(), input_lit.span(), span.local_file()) {
        Ok((vpath_hash, offset, length)) => {
            let expanded = quote::quote! {
                {
                    let mut buf = [0u8; #length as usize];
                    let vfs_instance = ::lib_core::vfs::VFS.get().expect("VFS не инициализирована!");

                    let file = &vfs_instance.open_files[&#vpath_hash];

                    #[cfg(target_os = "windows")]
                    use ::std::os::windows::fs::FileExt;
                    #[cfg(not(target_os = "windows"))]
                    use ::std::os::unix::fs::FileExt;

                    file.read_at(&mut buf, #offset).unwrap();
                    buf
                }
            };
            proc_macro::TokenStream::from(expanded)
        }
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}

#[proc_macro_attribute]
pub fn fill_to_4096(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Парсим ваш enum
    let mut input_enum = parse_macro_input!(item as ItemEnum);

    let mut max_id = 0u16;

    // Проверяем существующие варианты, чтобы узнать текущий максимальный ID
    for variant in &input_enum.variants {
        println!("dddddddddddddd 1: {}", max_id);
        if let Some((_, Expr::Lit(expr_lit))) = &variant.discriminant {
            println!("dddddddddddddd 2: {}", max_id);
            if let Lit::Int(lit_int) = &expr_lit.lit {
                println!("dddddddddddddd 3: {}", max_id);
                if let Ok(val) = lit_int.base10_parse::<u16>() {
                    println!("dddddddddddddd 4: {}", max_id);
                    if val > max_id {
                        max_id = val;
                        println!("dddddddddddddd 5: {}", max_id);
                    }
                }
            }
        }
    }

    // Если в enum уже есть блоки, резервировать начинаем со следующего ID
    let start_id = if input_enum.variants.is_empty() {
        0
    } else {
        max_id + 1
    };

    // Генерируем скрытые варианты _ReservedX от start_id до 4095
    for id in start_id..4096 {
        let name = format!("_Reserved{}", id);
        let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());

        let lit = syn::LitInt::new(&id.to_string(), proc_macro2::Span::call_site());
        let discriminant: Expr = syn::parse2(quote::quote! { #lit }).unwrap();

        // ИСПРАВЛЕНИЕ: Парсим атрибут через parse_outer
        let attrs = syn::Attribute::parse_outer
            .parse2(quote::quote! { #[allow(dead_code)] })
            .unwrap();

        let new_variant = syn::Variant {
            attrs, // Передаем вектор атрибутов напрямую
            ident,
            fields: Fields::Unit,
            discriminant: Some((syn::token::Eq::default(), discriminant)),
        };

        input_enum.variants.push(new_variant);
    }

    // Возвращаем измененный enum обратно в компилятор
    TokenStream::from(quote::quote! {
        #input_enum
    })
}

// #[proc_macro]
// pub fn vfs_read_at(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     let parser =
//         |input: syn::parse::ParseStream| -> syn::Result<(syn::LitStr, syn::Expr, syn::Expr)> {
//             let path = input.parse::<syn::LitStr>()?;
//             input.parse::<syn::Token![,]>()?;
//             let inner_offset = input.parse::<syn::Expr>()?;
//             input.parse::<syn::Token![,]>()?;
//             let inner_len = input.parse::<syn::Expr>()?;
//             Ok((path, inner_offset, inner_len))
//         };

//     let (input_lit, inner_offset, inner_len) = match parser.parse(input) {
//         Ok(res) => res,
//         Err(e) => return TokenStream::from(e.to_compile_error()),
//     };

//     match find_vfs_entry(&input_lit.value(), input_lit.span()) {
//         Ok((vpath_hash, base_offset, asset_length)) => {
//             if let (syn::Expr::Lit(offset_lit), syn::Expr::Lit(len_lit)) =
//                 (&inner_offset, &inner_len)
//             {
//                 if let (syn::Lit::Int(offset_int), syn::Lit::Int(len_int)) =
//                     (&offset_lit.lit, &len_lit.lit)
//                 {
//                     let req_offset = offset_int.base10_parse::<u64>().unwrap_or(0);
//                     let req_len = len_int.base10_parse::<u64>().unwrap_or(0);

//                     if req_offset + req_len > asset_length {
//                         return TokenStream::from(
//                             syn::Error::new(
//                                 input_lit.span(),
//                                 format!(
//                                     "Ошибка сборки: Выход за границы! Размер: {}, затребовано: {}",
//                                     asset_length,
//                                     req_offset + req_len
//                                 ),
//                             )
//                             .to_compile_error(),
//                         );
//                     }
//                 }
//             }

//             let expanded = quote::quote! {
//                 {
//                     let start = (#base_offset + (#inner_offset)) as u64;
//                     let mut buf = [0u8; #inner_len as usize];

//                     let vfs_instance = ::lib_core::vfs::VFS.get().expect("VFS не инициализирована!");
//                     let file = &vfs_instance.open_files[&#vpath_hash];

//                     #[cfg(target_os = "windows")]
//                     use ::std::os::windows::fs::FileExt;
//                     #[cfg(not(target_os = "windows"))]
//                     use ::std::os::unix::fs::FileExt;

//                     file.read_at(&mut buf, start).unwrap();
//                     buf
//                 }
//             };
//             proc_macro::TokenStream::from(expanded)
//         }
//         Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
//     }
// }

// #[proc_macro]
// pub fn vfs_read_at_vec(input: TokenStream) -> TokenStream {
//     let parser =
//         |input: syn::parse::ParseStream| -> syn::Result<(syn::LitStr, syn::Expr, syn::Expr)> {
//             let path = input.parse::<syn::LitStr>()?;
//             input.parse::<syn::Token![,]>()?;
//             let inner_offset = input.parse::<syn::Expr>()?;
//             input.parse::<syn::Token![,]>()?;
//             let inner_len = input.parse::<syn::Expr>()?;
//             Ok((path, inner_offset, inner_len))
//         };

//     let (input_lit, inner_offset, inner_len) = match parser.parse(input) {
//         Ok(res) => res,
//         Err(e) => return TokenStream::from(e.to_compile_error()),
//     };

//     match find_vfs_entry(&input_lit.value(), input_lit.span()) {
//         Ok((vpath_hash, base_offset, asset_length)) => {
//             // Пытаемся спарсить аргументы как числа на этапе компиляции, чтобы проверить их
//             if let (syn::Expr::Lit(offset_lit), syn::Expr::Lit(len_lit)) =
//                 (&inner_offset, &inner_len)
//             {
//                 if let (syn::Lit::Int(offset_int), syn::Lit::Int(len_int)) =
//                     (&offset_lit.lit, &len_lit.lit)
//                 {
//                     let req_offset = offset_int.base10_parse::<u64>().unwrap_or(0);
//                     let req_len = len_int.base10_parse::<u64>().unwrap_or(0);

//                     // Если константный запрос превышает размер ассета — бьем по рукам ПРИ КОМПИЛЯЦИИ
//                     if req_offset + req_len > asset_length {
//                         return TokenStream::from(syn::Error::new(
//                             input_lit.span(),
//                             format!(
//                                 "Ошибка сборки: Попытка выйти за границы файла! Размер ассета: {} байт, затребовано: {}",
//                                 asset_length, req_offset + req_len
//                             )
//                         ).to_compile_error());
//                     }
//                 }
//             }

//             // Если аргументы динамические (переменные из рантайма) — генерируем код дальше...
//             let expanded = quote::quote! {
//             {
//                 let start = (#base_offset + (#inner_offset)) as u64;
//                 let len = (#inner_len) as usize;

//                 // Создаем динамический вектор нужной длины
//                 let mut buf = vec![0u8; len];

//                 let vfs_instance = ::lib_core::vfs::VFS.get().expect("VFS не инициализирована!");
//                 let file = &vfs_instance.open_files[&#vpath_hash];

//                 #[cfg(target_os = "windows")]
//                 use ::std::os::windows::fs::FileExt;
//                 #[cfg(not(target_os = "windows"))]
//                 use ::std::os::unix::fs::FileExt;

//                 file.read_at(&mut buf, start).unwrap();
//                 buf // Возвращает Vec<u8>
//             }
//             };
//             TokenStream::from(expanded)
//         }
//         Err(compile_error) => TokenStream::from(compile_error.to_compile_error()),
//     }
// }

// #[proc_macro]
// pub fn vfs_slice_all(input: TokenStream) -> TokenStream {
//     let input_lit = parse_macro_input!(input as LitStr);

//     // Используем вашу общую функцию для поиска (она возвращает хэш, смещение и длину)
//     match find_vfs_entry(&input_lit.value(), input_lit.span()) {
//         Ok((vpath_hash, offset, length)) => {
//             // Генерируем супер-быстрый срез из mmap
//             let expanded = quote::quote! {
//                 {
//                     // Достаем глобальный статик из вашего нового пути lib_core
//                     let vfs_instance = ::lib_core::vfs::VFS.get()
//                         .expect("VFS не инициализирована!");

//                     // Находим mmap по u64 хэшу
//                     let mmap_ptr = &vfs_instance.mmaps[&#vpath_hash];

//                     // Возвращаем чистый срез байт. Ноль аллокаций, ноль проверок границ в рантайме.
//                     &mmap_ptr[#offset as usize .. (#offset + #length) as usize]
//                 }
//             };
//             proc_macro::TokenStream::from(expanded)
//         }
//         Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
//     }
// }

// #[proc_macro]
// pub fn vfs_slice_at(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     // Парсим три аргумента, разделенные запятой: (LitStr, Expr, Expr)
//     let parser =
//         |input: syn::parse::ParseStream| -> syn::Result<(syn::LitStr, syn::Expr, syn::Expr)> {
//             let path = input.parse::<syn::LitStr>()?; // VPath
//             input.parse::<syn::Token![,]>()?; // ,
//             let inner_offset = input.parse::<syn::Expr>()?; // Смещение
//             input.parse::<syn::Token![,]>()?; // ,
//             let inner_len = input.parse::<syn::Expr>()?; // Длина
//             Ok((path, inner_offset, inner_len))
//         };

//     let (input_lit, inner_offset, inner_len) = match parser.parse(input) {
//         Ok(res) => res,
//         Err(e) => return TokenStream::from(e.to_compile_error()),
//     };

//     // Ищем базовые координаты файла в vfs.meta на этапе компиляции
//     match find_vfs_entry(&input_lit.value(), input_lit.span()) {
//         Ok((vpath_hash, base_offset, asset_length)) => {
//             // Пытаемся спарсить аргументы как числа на этапе компиляции, чтобы проверить их
//             if let (syn::Expr::Lit(offset_lit), syn::Expr::Lit(len_lit)) =
//                 (&inner_offset, &inner_len)
//             {
//                 if let (syn::Lit::Int(offset_int), syn::Lit::Int(len_int)) =
//                     (&offset_lit.lit, &len_lit.lit)
//                 {
//                     let req_offset = offset_int.base10_parse::<u64>().unwrap_or(0);
//                     let req_len = len_int.base10_parse::<u64>().unwrap_or(0);

//                     // Если константный запрос превышает размер ассета — бьем по рукам ПРИ КОМПИЛЯЦИИ
//                     if req_offset + req_len > asset_length {
//                         return TokenStream::from(syn::Error::new(
//                         input_lit.span(),
//                         format!(
//                             "Ошибка сборки: Попытка выйти за границы файла! Размер ассета: {} байт, затребовано: {}",
//                             asset_length, req_offset + req_len
//                         )
//                     ).to_compile_error());
//                     }
//                 }
//             }

//             // Если аргументы динамические (переменные из рантайма) — генерируем код дальше...
//             let expanded = quote::quote! {
//             unsafe {
//                 let vfs_instance = ::lib_core::vfs::VFS.get().expect("VFS не инициализирована!");
//                 let mmap_ptr = &vfs_instance.mmaps[&#vpath_hash];

//                 let start = (#base_offset + (#inner_offset)) as usize;
//                 let len = (#inner_len) as usize;

//                 // Магия: берем указатель на начало пака, сдвигаем на старт
//                 // и создаем срез БЕЗ рантайм-проверок границ через std::slice::from_raw_parts
//                 let ptr = mmap_ptr.as_ptr().add(start);
//                 ::std::slice::from_raw_parts(ptr, len)
//                 }
//             };
//             TokenStream::from(expanded)
//         }
//         Err(compile_error) => TokenStream::from(compile_error.to_compile_error()),
//     }
// }

#[proc_macro]
pub fn vfs_include_str(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    let resolve_path = || {
        let workspace_root = get_workspace_dir_from(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .map_err(|err| syn::Error::new(input_lit.span(), format!("Workspace error: {}", err)))?;

        init_builder_state(&workspace_root).map_err(|err| {
            syn::Error::new(input_lit.span(), format!("Builder init error: {}", err))
        })?;

        let span = proc_macro::Span::call_site();

        lib_core::fs::vfs::resolve::resolve_path_checked(
            &input_lit.value(),
            &workspace_root,
            span.local_file(),
        )
        .map_err(|err| syn::Error::new(input_lit.span(), format!("Resolve error: {}", err)))
    };

    match resolve_path() {
        Ok(path) => match path.to_str() {
            Some(path_str) => {
                let expanded = quote::quote! {
                    include_str!(#path_str)
                };
                proc_macro::TokenStream::from(expanded)
            }
            None => {
                let err =
                    syn::Error::new(input_lit.span(), "Path contains invalid UTF-8 characters");
                proc_macro::TokenStream::from(err.to_compile_error())
            }
        },
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}

#[proc_macro]
pub fn vfs_include_bytes(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    let resolve_path = || {
        let workspace_root = get_workspace_dir_from(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .map_err(|err| syn::Error::new(input_lit.span(), format!("Workspace error: {}", err)))?;

        init_builder_state(&workspace_root).map_err(|err| {
            syn::Error::new(input_lit.span(), format!("Builder init error: {}", err))
        })?;

        let span = proc_macro::Span::call_site();

        lib_core::fs::vfs::resolve::resolve_path_checked(
            &input_lit.value(),
            &workspace_root,
            span.local_file(),
        )
        .map_err(|err| syn::Error::new(input_lit.span(), format!("Resolve error: {}", err)))
    };

    match resolve_path() {
        Ok(path) => match path.to_str() {
            Some(path_str) => {
                let expanded = quote::quote! {
                    include_bytes!(#path_str)
                };
                proc_macro::TokenStream::from(expanded)
            }
            None => {
                let err =
                    syn::Error::new(input_lit.span(), "Path contains invalid UTF-8 characters");
                proc_macro::TokenStream::from(err.to_compile_error())
            }
        },
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}

// #[proc_macro]
// pub fn include_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//     // 1. Получаем виртуальный путь на этапе компиляции (например, "/packs/ui")
//     let input_litstr = parse_macro_input!(input as LitStr);
//     let input_str = input_litstr.value();

//     // 2. Читаем include_config.toml с диска
//     let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Не найдена корневая папка крейта");
//     let config_path = PathBuf::from(manifest_dir).join("include_config.toml");

//     let toml_str = fs::read_to_string(&config_path).expect(&format!(
//         "Не удалось прочитать файл: {}",
//         config_path.display()
//     ));

//     // 3. Парсим динамическую таблицу (количество полей неизвестно)
//     let table: Table = toml_str.parse::<Table>().expect(&format!(
//         "Не удалось распарсить файл: {}",
//         config_path.display()
//     ));

//     // 4. Ищем ключ в таблице прямо СЕЙЧАС (до компиляции)
//     if let Some(value) = table.get(&input_str) {
//         if let toml::Value::String(real_path) = value {
//             let expanded = quote::quote! {
//                 {
//                     #real_path
//                 }
//             };
//             return TokenStream::from(expanded);
//         }
//     }

//     // 5. Если разработчик ошибся в названии пути — компиляция завершится ошибкой!
//     let error_msg = format!("Поле '{}' не найдено", input_str);
//     syn::Error::new_spanned(input_litstr, error_msg)
//         .to_compile_error()
//         .into()
// }
