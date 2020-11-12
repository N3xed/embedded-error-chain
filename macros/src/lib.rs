//! Derive macros for embedded-error-chain.

#![crate_type = "proc-macro"]
#![allow(broken_intra_doc_links)]

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, DeriveInput};
mod error_category;
mod str_placeholder;

#[proc_macro_error]
#[proc_macro_derive(ErrorCategory, attributes(error_category, error))]
pub fn error_category(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    error_category::derive_error_category(input).into()
}
