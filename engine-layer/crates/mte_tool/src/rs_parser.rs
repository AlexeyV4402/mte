use std::fs;
use std::path::PathBuf;

use regex::Regex;
use syn::parse::{Parse, ParseStream};
use syn::visit::Visit;
use syn::{Expr, Ident, LitBool, LitStr, Token, parse_macro_input};

struct BlockDefinition {
    name: Ident,
    solid: LitBool,
    shape: Expr,
    profile: Ident,
    textures: Vec<LitStr>,
}

// Реализуем трейт Parse, чтобы syn знал, как читать наш синтаксис:
// Air => { solid: false, shape: Shape::None, profile: AllSides, textures: [...] }
impl Parse for BlockDefinition {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;

        let content;
        syn::braced!(content in input);

        // Читаем solid
        content.parse::<Ident>()?; // пропускаем "solid"
        content.parse::<Token![:]>()?;
        let solid: LitBool = content.parse()?;
        content.parse::<Token![,]>()?;

        // Читаем shape
        content.parse::<Ident>()?; // пропускаем "shape"
        content.parse::<Token![:]>()?;
        let shape: Expr = content.parse()?;
        content.parse::<Token![,]>()?;

        // Читаем profile
        content.parse::<Ident>()?; // пропускаем "profile"
        content.parse::<Token![:]>()?;
        let profile: Ident = content.parse()?;
        content.parse::<Token![,]>()?;

        // Читаем textures
        content.parse::<Ident>()?; // пропускаем "textures"
        content.parse::<Token![:]>()?;

        let tex_array;
        syn::bracketed!(tex_array in content);
        let mut textures = Vec::new();
        while !tex_array.is_empty() {
            let tex: LitStr = tex_array.parse()?;
            textures.push(tex);
            if tex_array.is_empty() {
                break;
            }
            tex_array.parse::<Token![,]>()?;
        }

        // Разрешаем опциональную запятую в конце структуры блока
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }

        Ok(BlockDefinition {
            name,
            solid,
            shape,
            profile,
            textures,
        })
    }
}

// Контейнер для ВСЕХ блоков, разделенных запятыми
struct BlocksList {
    blocks: Vec<BlockDefinition>,
}

impl Parse for BlocksList {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let mut blocks = Vec::new();
        while !input.is_empty() {
            blocks.push(input.parse()?);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(BlocksList { blocks })
    }
}

struct VfsAssetFinder {
    pub current_file: std::path::PathBuf,
    pub found_dependencies: Vec<(String, std::path::PathBuf)>,
}

impl<'ast> syn::visit::Visit<'ast> for VfsAssetFinder {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("vfs_read") {
            if let Ok(lit) = mac.parse_body::<syn::LitStr>() {
                let vpath = lit.value();

                // Связываем ассет с файлом, в котором он был найден!
                let dependency = (vpath, self.current_file.clone());

                if !self.found_dependencies.contains(&dependency) {
                    self.found_dependencies.push(dependency);
                }
            }
        } else if mac.path.is_ident("define_blocks") {
            if let Ok(input_list) = mac.parse_body::<BlocksList>() {
                let mut all_textures = Vec::new();

                for block in input_list.blocks {
                    for tex in &block.textures {
                        all_textures.push(tex.value());
                    }
                }

                for path in all_textures {
                    let dependency = (path, self.current_file.clone());

                    // if !self.found_dependencies.contains(&dependency) {
                    self.found_dependencies.push(dependency);
                    // }
                }
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

#[inline]
pub fn run_regex_parser(files: impl Iterator<Item = std::path::PathBuf>) -> Vec<(String, PathBuf)> {
    let mut found_paths = Vec::new();
    let re = Regex::new(r#"vfs_read!\(\s*"([^"]+)"\s*\)"#).unwrap();

    for file_path in files {
        if let Ok(code) = fs::read_to_string(&file_path) {
            for caps in re.captures_iter(&code) {
                let vpath_slice = &caps[1];

                let already_exists = found_paths
                    .iter()
                    .any(|(v, f)| v == vpath_slice && f == &file_path);

                if !already_exists {
                    found_paths.push((vpath_slice.to_string(), file_path.clone()));
                }
            }
        }
    }

    found_paths
}

#[inline]
pub fn run_syn_parser(files: impl Iterator<Item = std::path::PathBuf>) -> Vec<(String, PathBuf)> {
    let mut finder = VfsAssetFinder {
        current_file: std::path::PathBuf::new(),
        found_dependencies: Vec::new(),
    };

    for file_path in files {
        if let Ok(code) = std::fs::read_to_string(&file_path) {
            if let Ok(ast) = syn::parse_file(&code) {
                finder.current_file = file_path;

                finder.visit_file(&ast);
            }
        }
    }

    finder.found_dependencies
}
