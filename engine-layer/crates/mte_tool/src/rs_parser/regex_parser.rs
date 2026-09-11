use std::fs;
use std::path::PathBuf;

use regex::Regex;

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
