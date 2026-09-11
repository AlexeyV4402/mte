use proc_macro::TokenStream;
use syn::{LitStr, parse_macro_input};

use crate::macro_core::resolve_path;

pub fn vfs_include_bytes_impl(input: proc_macro::TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    match resolve_path(&input_lit, lib_core::fs::vfs::resolve::resolve_path_checked) {
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

pub fn vfs_include_str_impl(input: proc_macro::TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as LitStr);

    match resolve_path(&input_lit, lib_core::fs::vfs::resolve::resolve_path_checked) {
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
