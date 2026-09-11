use lib_core::fs::os::{get_workspace_dir, get_workspace_dir_from};
use lib_core::fs::vfs::resolve::get_cache_path;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Expr, Fields, Ident, ItemEnum, Lit, LitBool, LitStr, Token, parse_macro_input};

use crate::macro_core::resolve_path;

pub fn vfs_include_vk_shader_impl(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    match resolve_path(&input_lit, lib_core::fs::vfs::resolve::resolve_path_checked) {
        Ok(path) => {
            let cache_path = match get_cache_path(&path) {
                Ok(cache_path) => cache_path,
                Err(err) => {
                    return proc_macro::TokenStream::from(
                        syn::Error::new(input_lit.span(), err).to_compile_error(),
                    );
                }
            };

            let string_path = cache_path.to_string_lossy().to_string();

            let expanded = quote::quote! {
                {
                    include_bytes!(#string_path)
                }
            };
            proc_macro::TokenStream::from(expanded)
        }
        Err(compile_error) => proc_macro::TokenStream::from(compile_error.to_compile_error()),
    }
}
