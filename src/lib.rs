#![no_std]

mod error;
mod error_category;
mod error_data;

pub use error::{ChainError, Error, ErrorIter, ResultChainError};
pub use error_category::{
    format_chained, get_next_formatter, ErrorCategory, ErrorCategoryHandle, ErrorCodeFormatter,
    ErrorCodeFormatterVal,
};
pub use error_data::{ErrorData, ERROR_CHAIN_LEN};

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

/// An error code.
///
/// Must only be 4 bits wide.
pub type ErrorCode = u8;
