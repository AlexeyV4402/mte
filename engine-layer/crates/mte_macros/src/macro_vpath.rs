use std::path::{Path, PathBuf};

use lib_core::fs::vfs::resolve::VfsResolverError;
use proc_macro::TokenStream;
use syn::{LitStr, parse_macro_input};

use crate::macro_core::resolve_path;

pub(crate) fn vpath_impl<F>(input: TokenStream, resolver: F) -> TokenStream
where
    F: Fn(&str, &Path, Option<PathBuf>) -> Result<PathBuf, VfsResolverError>,
{
    let input_lit = parse_macro_input!(input as LitStr);

    match resolve_path(&input_lit, resolver) {
        Ok(path) => {
            let components: Vec<&str> = path
                .components()
                .map(|c| c.as_os_str().to_str().unwrap())
                .collect();
            let expanded = quote::quote! {
                {
                    [#(#components),*].iter().collect::<std::path::PathBuf>()
                }
            };
            proc_macro::TokenStream::from(expanded)
        }
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}
