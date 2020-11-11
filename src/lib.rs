#![no_std]
#![cfg_attr(feature = "nightly", feature(const_panic, const_fn))]

mod error;
mod error_category;
mod error_data;

pub use error::{ChainError, Error, ErrorIter, ResultChainError};
pub use error_category::{
    format_chained, get_next_formatter, ErrorCategory, ErrorCategoryHandle, ErrorCodeFormatter,
    ErrorCodeFormatterVal,
};
pub use error_data::{ErrorData, ERROR_CHAIN_LEN};
pub use macros::ErrorCategory;

pub mod prelude {
    pub use super::{ChainError, Error, ErrorCategory, ErrorCategoryHandle, ResultChainError};
}

/// Marker types.
pub mod marker {
    /// A tag type to disambiguate between trait implementations.
    pub struct C0;
    /// A tag type to disambiguate between trait implementations.
    pub struct C1;
    /// A tag type to disambiguate between trait implementations.
    pub struct C2;
    /// A tag type to disambiguate between trait implementations.
    pub struct C3;

    pub use super::error_category::Unused;
}

/// Utilities used by the proc-macro.
pub mod utils {
    #[cfg(feature = "nightly")]
    pub const fn const_assert(cond: bool, msg: &'static str) {
        assert!(cond, msg);
    }

    #[cfg(feature = "nightly")]
    #[macro_export]
    macro_rules! const_assert {
        ($cond:expr, $msg:expr) => {
            const _: () = $crate::utils::const_assert($cond, $msg);
        };
    }

    #[cfg(not(feature = "nightly"))]
    #[macro_export]
    macro_rules! const_assert {
        ($cond:expr, $msg:expr) => {
            $crate::utils::const_assert!($cond);
        };
    }

    pub use static_assertions::*;
}

/// An error code.
///
/// Must only be 4 bits wide.
pub type ErrorCode = u8;
