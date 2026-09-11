use std::path::{Path, PathBuf};

use lib_core::fs::os::get_workspace_dir_from;
use lib_core::fs::vfs::builder::init_builder_state;
use lib_core::fs::vfs::resolve::VfsResolverError;
use syn::LitStr;

pub fn resolve_path<F>(input_lit: &LitStr, resolver: F) -> Result<PathBuf, syn::Error>
where
    F: Fn(&str, &Path, Option<PathBuf>) -> Result<PathBuf, VfsResolverError>,
{
    let workspace_root = get_workspace_dir_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .map_err(|err| {
        syn::Error::new(input_lit.span(), format!("Workspace error: {}", err))
    })?;

    init_builder_state(&workspace_root)
        .map_err(|err| syn::Error::new(input_lit.span(), format!("Builder init error: {}", err)))?;

    let span = proc_macro::Span::call_site();

    resolver(&input_lit.value(), &workspace_root, span.local_file())
        .map_err(|err| syn::Error::new(input_lit.span(), format!("Resolve error: {}", err)))
}
