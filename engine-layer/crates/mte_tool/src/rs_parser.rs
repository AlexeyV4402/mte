use std::fs;
use std::path::PathBuf;

use regex::Regex;
use syn::visit::Visit;

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
