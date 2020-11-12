/*!
[![Crates.io](https://img.shields.io/crates/v/embedded-error-chain.svg)](https://crates.io/crates/embedded-error-chain)
[![API reference](https://docs.rs/embedded-error-chain/badge.svg)](https://docs.rs/embedded-error-chain/)

Easy error handling for embedded devices (`no_alloc` and `no_std`).

A rust library implementing easy error handling for embedded devices. An [`Error`] value is
only a single [`u32`] in size and supports up to 4 chained error codes. Each error code can
have a value from `0` to `15` (4 bits). All error codes come from an enum that implements
the [`ErrorCategory`] trait (a derive macro exists). This trait is also used to implement
debug printing and equality for each error code.

This library was inspired by libraries such as [error-chain](https://crates.io/crates/error-chain)
and [anyhow](https://crates.io/crates/anyhow), though its goal is to work in `no_std` and `no_alloc`
environments with very little memory overhead.

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

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(GyroAccError))]
#[repr(u8)]
enum CalibrationError {
    Inner,
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

fn calibrate() -> Result<(), Error<CalibrationError>> {
    gyro_acc_init().chain_err(CalibrationError::Inner)?;
    Ok(())
}
```
*/

#![no_std]
#![cfg_attr(feature = "nightly", feature(const_panic, const_fn))]

#[cfg(feature = "std")]
extern crate std;

mod error;
mod error_category;
mod error_data;

#[doc(hidden)]
pub mod utils;

pub use error::{ChainError, Error, ErrorIter, ResultChainError};
pub use error_category::{
    format_chained, ErrorCategory, ErrorCategoryHandle, ErrorCodeFormatter, ErrorCodeFormatterVal,
};
pub use error_data::{ErrorData, ERROR_CHAIN_LEN};

/// Everything for easy error handling.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{ChainError, Error, ErrorCategory, ErrorCategoryHandle, ResultChainError};
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

    pub use super::error_category::Unused;
}

/// An error code.
///
/// Must only be 4 bits wide.
pub type ErrorCode = u8;

/// Derive [`ErrorCategory`](ErrorCategory) for an enum.
///
/// This will also try to derive the trait dependencies of
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
/// ## Usage
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
pub use macros::ErrorCategory;
