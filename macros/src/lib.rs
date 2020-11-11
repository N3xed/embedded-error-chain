//! Derive macros for embedded-error-chain

#![crate_type = "proc-macro"]
#![allow(dead_code)]

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, DeriveInput};
mod error_category;
mod str_placeholder;

/// Derive `ErrorCategory` for an enum.
///
/// ## Usage
///
/// ```rust
/// use embedded_error_chain::prelude::*;
///
/// #[derive(ErrorCategory)]
/// enum OtherError {
///     ExtremeFailure,
/// }
///
/// static SOME_GLOBAL_VARIABLE: usize = 200;
///
/// #[derive(ErrorCategory)]
/// #[error_category(name = "optional name", links(OtherError))]
/// enum TestError {
///     #[error("format string {summary}, {details}, {variant}, {category}")]
///     Foo = 0,
///
///     #[error("custom {}, {:?}", "some_expr", SOME_GLOBAL_VARIABLE)]
///     Other,
///
///     /// Summary
///     ///
///     /// Detailed description.
///     /// The summary and detailed description are available as placeholders in
///     /// the `#[error(...)]` attribute. If no such attribute is put on the variant
///     /// or the `...` part is empty, then the summary will be used. If the summary
///     /// does not exist (no doc comments on the variant), then the variant name is
///     /// used for debug printing.
///     Bar,
/// }
///
/// #[derive(ErrorCategory)]
/// #[error_category(links(OtherError, TestError))]
/// enum SeperateError {
///     SomethingHappened,
/// }
///
/// let ec = ErrorCode::new(SeperateError::SomethingHappened)
///          .chain(OtherError::ExtremeFailure)
///          .chain(TestError::Foo);
///
/// println!("{:?}", ec);
/// let mut iter = ec.iter();
/// assert_eq!(
///     iter.next(),
///     Some((
///         TestError::Foo.into(),
///         ErrorCategoryHandle::new::<TestError>()
///     ))
/// );
/// assert_eq!(
///     iter.next(),
///     Some((
///         OtherError::ExtremeFailure.into(),
///         ErrorCategoryHandle::new::<OtherError>()
///     ))
/// );
/// assert_eq!(
///     iter.next(),
///     Some((
///         SeperateError::SomethingHappened.into(),
///         ErrorCategoryHandle::new::<SeperateError>()
///     ))
/// );
/// assert_eq!(iter.next(), None);
/// ```
#[proc_macro_error]
#[proc_macro_derive(ErrorCategory, attributes(error_category, error))]
pub fn error_category(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    error_category::derive_error_category(input).into()
}
