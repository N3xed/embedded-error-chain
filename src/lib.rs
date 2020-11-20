/*!
Easy error handling for embedded devices (no `liballoc` and `no_std`).

Errors are represented by error codes and come from enums that implement the
[`ErrorCategory`] trait (a derive macro exists), which is used for custom debug
printing per error code among other things. Each error code can have a value from `0`
to `15` (4 bits) and you can chain an error with up to four different error codes of
different categories.

The [`Error`] type encapsulates an error code and error chain, and is only a single
[`u32`] in size. There is also an untyped [`DynError`] type, which unlike [`Error`]
does not have a type parameter for the current error code. Its size is a [`u32`] +
pointer ([`usize`]), which can be used to forward source errors of different categories
to the caller.

This library was inspired by libraries such as
[error-chain](https://crates.io/crates/error-chain),
[anyhow](https://crates.io/crates/anyhow) and
[thiserror](https://crates.io/crates/thiserror), though it was made to work in `no_std`
**and** no `liballoc` environments with very little memory overhead.

## Example
```rust
use embedded_error_chain::prelude::*;

#[derive(Clone, Copy, ErrorCategory)]
#[repr(u8)]
enum SpiError {
    BusError,
    // ...
}

static LAST_GYRO_ACC_READOUT: usize = 200;

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(SpiError))]
#[repr(u8)]
enum GyroAccError {
    InitFailed,

    #[error("{variant} (readout={})", LAST_GYRO_ACC_READOUT)]
    ReadoutFailed,

    /// Value must be in range [0, 256)
    #[error("{variant}: {summary}")]
    InvalidValue,
}

fn main() {
    if let Err(err) = calibrate() {
        // log the error
        println!("{:?}", err);
        // ...
    }

    let readout = match gyro_acc_readout() {
        Ok(val) => val,
        Err(err) => {
            if let Some(spi_error) = err.code_of_category::<SpiError>() {
                // try to fix it
                0
            }
            else {
                panic!("unfixable spi error");
            }
        }
    };
}

fn spi_init() -> Result<(), SpiError> {
    Err(SpiError::BusError)
}

fn gyro_acc_init() -> Result<(), Error<GyroAccError>> {
    spi_init().chain_err(GyroAccError::InitFailed)?;
    Ok(())
}

fn gyro_acc_readout() -> Result<u32, Error<GyroAccError>> {
    Err(SpiError::BusError.chain(GyroAccError::InvalidValue))
}

fn calibrate() -> Result<(), DynError> {
    gyro_acc_init()?;
    // other stuff...
    Ok(())
}
```
*/

#![no_std]
#![cfg_attr(feature = "nightly", feature(const_panic, const_fn))]
#![warn(missing_docs)]
#![allow(clippy::clippy::trivially_copy_pass_by_ref)]

#[cfg(feature = "std")]
extern crate std;

mod dyn_error;
mod error;
mod error_category;
mod error_data;

#[doc(hidden)]
pub mod utils;

pub use dyn_error::DynError;
pub use error::{ChainError, Error, ErrorIter, ResultChainError};
pub use error_category::{
    format_chained, ErrorCategory, ErrorCategoryHandle, ErrorCodeFormatter, ErrorCodeFormatterVal,
};
pub use error_data::{ErrorData, ERROR_CHAIN_LEN};

/// Everything for easy error handling.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{
        ChainError, DynError, Error, ErrorCategory, ErrorCategoryHandle, ResultChainError,
    };
}

/// Marker types.
pub mod marker {
    /// A tag type to disambiguate between trait implementations.
    pub struct L0;
    /// A tag type to disambiguate between trait implementations.
    pub struct L1;
    /// A tag type to disambiguate between trait implementations.
    pub struct L2;
    /// A tag type to disambiguate between trait implementations.
    pub struct L3;
    /// A tag type to disambiguate between trait implementations.
    pub struct L4;
    /// A tag type to disambiguate between trait implementations.
    pub struct L5;

    /// A tag type to disambiguate between `ChainError` trait implementations for
    /// `Error<T>` and just for `T`.
    #[allow(non_camel_case_types)]
    pub struct Error_t;

    /// A tag type to disambiguate between `ChainError` trait implementations for
    /// `Error<T>` and just for `T`.
    #[allow(non_camel_case_types)]
    pub struct Concrete_t;

    pub use crate::error_category::Unused;
}

/// An error code belonging to an [`ErrorCategory`].
///
/// Must only be 4 bits wide.
pub type ErrorCode = u8;

/// Derive [`ErrorCategory`] for an enum.
///
/// This will also derive the trait dependencies of
/// [`ErrorCategory`](ErrorCategory) with the exception of
/// [`Copy`](core::marker::Copy):
/// - [`core::fmt::Debug`](core::fmt::Debug)
/// - [`Into`](core::convert::Into)`<`[`ErrorCode`](ErrorCode)`>`
/// - [`From`](core::convert::From)`<`[`ErrorCode`](ErrorCode)`>`
///
/// The traits `Into<ErrorCode>` and `From<ErrorCode>` are only derived if the enum is
/// `repr(u8)` (see
/// [`Data Layout` in the
/// nomicon](https://doc.rust-lang.org/nomicon/other-reprs.html#repru-repri)) or does *not*
/// contain any variants.
///
/// ## `#[error_category]` attribute
/// This attribute is optionally put once on the enum that is to be derived. It specifies
/// an optional [`ErrorCategory::NAME`] value (used for debug printing) and `0` to `6`
/// linked [`ErrorCategory`] types. If no `name` argument is given, the name of the enum
/// will be used for [`ErrorCategory::NAME`]. If no links are specified, the [error
/// category](ErrorCategory) is not linked.
///
/// **Example:**
/// ```
/// # use embedded_error_chain::prelude::*;
/// #
/// # #[derive(Clone, Copy, ErrorCategory)]
/// # enum Type0 {}
/// # #[derive(Clone, Copy, ErrorCategory)]
/// # enum Type1 {}
/// #
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[error_category(name = "CustomName", links(Type0, Type1))]
/// #[repr(u8)]
/// enum FooError {
///     Error,
/// }
/// ```
///
/// ## `#[error]` attribute
/// This attribute is also optional and can be placed once above every enum variant.
/// Its arguments specify the arguments used for debug printing of an error code
/// represented by the variant.
///
/// Everything inside the paranthese (`#[error(...)]`) will directly be used as the
/// arguments of the [`write!()`] macro. So the attribute `#[error("fmt string {} {}",
/// GLOBAL_VAL, 5)]` will be translated to `write!(f, "fmt string {} {}", GLOBAL_VAL, 5)`.
/// The first argument must be a string literal and can contain special placeholders that
/// will be replaced by the derive macro:
///
/// - `{category}` will be replaced with the value of [`ErrorCategory::NAME`].
/// - `{variant}` will be replaced with the name of the variant.
/// - `{details}` will be replaced with the details section of the doc comments on the variant.
/// - `{summary}` will be replaced with the summary of the doc comments on the variant.
///
/// The summary section of the doc comments is all non-empty lines, ignoring all empty
/// lines until the first non-empty line, until an empty line or the end of the doc
/// comments. All the summary lines are then trimmed for whitespace and joined using a
/// space character (` `).
///
/// The details section of the doc comments is all lines (empty and non-empty) with the
/// first whitespace removed after the summary section and ignoring all empty-lines until
/// the first non-empty line.
///
/// **Example:**
/// ```text
/// <summmary> /// Summary starts here...
///            /// some more summary
/// </summary> /// ...and ends here.
///            ///
///            ///
/// <details>  /// Details start here...
///            ///
///            /// more details
/// </details> /// ...and end here.
/// ```
/// - summary:  
///     > ```text
///     > Summary starts here... some more summary ...and ends here.
///     > ```
///
/// - details:
///     > ```text
///     > Details start here...
///     >
///     > more details
///     > ...and end here.
///     > ```
///
/// If no `#[error]` attribute is put on the variant, then the summary part of the doc
/// comments will be used (see above). If the summary does not exist (no doc comments on
/// the variant) or is empty, then the variant name is used for debug printing.
///
/// ## Full example
///
/// ```rust
/// use embedded_error_chain::prelude::*;
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum OtherError {
///     ExtremeFailure,
/// }
///
/// static SOME_GLOBAL_VARIABLE: usize = 200;
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[error_category(name = "optional name", links(OtherError))]
/// #[repr(u8)]
/// enum TestError {
///     /// Foo error (summary)
///     ///
///     /// Detailed description.
///     /// The summary and detailed description are available as placeholders in
///     /// the `#[error(...)]` attribute. If no such attribute is put on the variant
///     /// or the `...` part is empty, then the summary will be used. If the summary
///     /// does not exist (no doc comments on the variant), then the variant name is
///     /// used for debug printing.
///     #[error("format string {summary}, {details}, {variant}, {category}")]
///     Foo = 0,
///
///     #[error("custom {}, {:?}", "some_expr", SOME_GLOBAL_VARIABLE)]
///     Other,
///
///     /// Some explanation explanation
///     Bar,
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[error_category(links(OtherError, TestError))]
/// #[repr(u8)]
/// enum SeperateError {
///     SomethingHappened,
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// enum YetEmptyError {}
/// ```
pub use embedded_error_chain_macros::ErrorCategory;
