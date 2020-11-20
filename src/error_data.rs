#[allow(unused_imports)]
use crate::ErrorCategory;
use crate::ErrorCode;

/// The maximum amount of error codes that can be chained to an [`Error`](crate::Error) or
/// [`DynError`](crate::DynError).
///
/// This is equal to the amount of [`ErrorData::chain()`] (or
/// [`ChainError::chain()`](super::ChainError::chain()),
/// [`ResultChainError::chain_err()`](super::ResultChainError::chain_err())) you can make
/// before the chain overflows, and it either panics (if the feature `panic-on-overflow`
/// is enabled) or the oldest error code gets lost.
pub const ERROR_CHAIN_LEN: usize = 4;
/// The entire data of the error and its error code chain.
///
/// This is a wrapper over a bit-packed [`u32`] value that contains five 4-bit wide
/// [`ErrorCode`](crate::ErrorCode)s and four 3-bit wide
/// [`ErrorCodeFormatter`](crate::ErrorCodeFormatter) indices.
///
/// The bit layout of the underlying `u32` value is a follows:
/// - Bits `b0..b20` contain 5 error codes, each error code is 4 bits.
///   - `b0..b4`: the error code of the current error (returned by [`code()`](Self::code()))
///   - `b4..b8`: chained error code 0
///   - `b8..b12`: chained error code 1
///   - `b12..b16`: chained error code 2
///   - `b16..b20`: chained error code 3
/// - Bits `b20..b32` contain 4 formatter indices, each index has 3 bits.
///   - `b20..b23`: formatter `index + 1` of chained error 0 (`0` means not present)
///                 (returned by [`first_formatter_index()`](Self::first_formatter_index()))
///   - `b23..b26`: formatter `index + 1` of chained error 1 (`0` means not present)
///   - `b26..b29`: formatter `index + 1` of chained error 2 (`0` means not present)
///   - `b29..b32`: formatter `index + 1` of chained error 3 (`0` means not present)
///
/// The first [error code](crate::ErrorCode) represents the most recent or current error.
/// The next four [error codes](crate::ErrorCode) with the formatter indices represent the
/// error chain which can be empty. The error chain (as described in the documentation of
/// [`Error`](crate::Error)) is a singly linked list. As much of the data used for error
/// reporting is constant or static, so that no dynamic allocation is needed, to make
/// runtime memory usage as small as possible and to make it cheap to copy an error value
/// around. This is also the case with the error chain.
///
/// Every [`ErrorCode`] value belongs to a type that implements the trait
/// [`ErrorCategory`]. Using this trait it is possible to print a custom name and
/// additional information for every [`ErrorCode`] value. Only the [`ErrorCategory`] of
/// the most recent error code has to be known, all other [error
/// categories](ErrorCategory) can then be retrieved by iterating over the linked list.
/// The [`ErrorCategory`] is also needed for the linked list to be possible.
///
/// Every formatter index in the chain represents the
/// [`ErrorCodeFormatter`](crate::ErrorCodeFormatter) function of the [error
/// category](ErrorCategory) and error code. A formatter function is retrieved by calling
/// the formatter function of the previous error code, and passing it the index of the
/// next formatter function. The called formatter function gets the next formatter
/// function from from the slice returned by
/// [`ErrorCategory::chainable_category_formatters()`] using the next formatter index.
/// This is only possible if the [`ErrorCategory`] associated with the called formatter
/// function is linked to the [`ErrorCategory`] of the next error code in the chain.
///
/// An [error category](ErrorCategory) `A` is linked to an [error category](ErrorCategory)
/// `B` if at least one of the [`A::L1`](ErrorCategory::L1) to
/// [`A::L5`](ErrorCategory::L1) associated types is `B` and the `n`th element (where `n`
/// is the digit of the [`A::Ln`](ErrorCategory::L1) associated type used) of the slice
/// returned by
/// [`A::chainable_category_formatters()`](ErrorCategory::chainable_category_formatters())
/// is the [`ErrorCodeFormatter`](crate::ErrorCodeFormatter) function for `B`.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorData {
    /// Contains the entire data of the error and its error code chain.
    ///
    /// - Bits `b0..b20` contain 5 error codes, each error code is 4 bits.
    ///   - `b0..b4`: the error code of the current error (returned by `Self::code()`)
    ///   - `b4..b8`: chained error code 0
    ///   - `b8..b12`: chained error code 1
    ///   - `b12..b16`: chained error code 2
    ///   - `b16..b20`: chained error code 3
    /// - Bits `b20..b32` contain 4 formatter indices, each index has 3 bits.
    ///   - `b20..b23`: formatter `index + 1` of chained error 0 (`0` means not present)
    ///                 (returned by `Self::first_formatter_index()`)
    ///   - `b23..b26`: formatter `index + 1` of chained error 1 (`0` means not present)
    ///   - `b26..b29`: formatter `index + 1` of chained error 2 (`0` means not present)
    ///   - `b29..b32`: formatter `index + 1` of chained error 3 (`0` means not present)
    data: u32,
}

mod consts {
    pub const CODE_MASK: [u32; 5] = [
        0x0000_000f,
        0x0000_00f0,
        0x0000_0f00,
        0x0000_f000,
        0x000f_0000,
    ];
    pub const ALL_CODE_MASK: u32 = 0x000f_ffff;
    /// A error code has 4 bits.
    pub const CODE_WIDTH: u32 = 4;

    #[inline(always)]
    pub const fn make_code(value: super::ErrorCode) -> u32 {
        (value & 0b1111) as u32
    }

    pub const FORMATTER_MASK: [u32; 4] = [0x0070_0000, 0x0380_0000, 0x1c00_0000, 0xe000_0000];
    pub const ALL_FORMATTER_MASK: u32 = 0xfff0_0000;
    /// The first formatter index begins at bit 20.
    pub const FORMATTER_BITOFFSET: u32 = 20;
    /// A formatter index has 3 bits.
    pub const FORMATTER_IDX_WIDTH: u32 = 3;

    #[inline(always)]
    pub const fn make_formatter_idx(value: u8) -> u32 {
        (value & 0b0111) as u32
    }
}

impl ErrorData {
    /// Create new `ErrorData` that contains the supplied `error_code` and has an empty chain.
    pub const fn new(error_code: ErrorCode) -> ErrorData {
        ErrorData {
            data: error_code as u32 & consts::CODE_MASK[0],
        }
    }

    /// Replace the error code with `code` and return the old one.     
    ///
    /// Note: That the categories of the new error code and the old must be the same.
    pub fn set_code(&mut self, code: ErrorCode) -> ErrorCode {
        let old_ec = consts::make_code(self.data as u8) as ErrorCode;
        self.data = (self.data & !(consts::CODE_MASK[0])) | consts::make_code(code);
        old_ec
    }

    /// Get the most recent error code of the error.
    #[inline]
    pub fn code(&self) -> ErrorCode {
        (self.data & consts::CODE_MASK[0]) as ErrorCode
    }

    /// Get the first formatter index in the chain if available.
    pub fn first_formatter_index(&self) -> Option<u8> {
        let fmt_index =
            ((self.data & consts::FORMATTER_MASK[0]) >> consts::FORMATTER_BITOFFSET) as u8;
        if fmt_index > 0 {
            Some(fmt_index - 1)
        } else {
            None
        }
    }

    /// Get the number of chained error codes.
    pub fn chain_len(&self) -> usize {
        // If the formatter is zero that means it is not present.
        let mut mask = consts::FORMATTER_MASK[0];

        for fmt_index in 0..ERROR_CHAIN_LEN {
            if (self.data & mask) == 0 {
                return fmt_index;
            }
            mask <<= consts::FORMATTER_IDX_WIDTH;
        }
        ERROR_CHAIN_LEN
    }

    /// Whether the error chain is full.
    #[inline]
    pub fn chain_full(&self) -> bool {
        self.chain_len() == ERROR_CHAIN_LEN
    }

    /// Prepend the current error code to the front of the error chain and set the current error
    /// code to `error_code`.
    ///
    /// Returns the back of the error chain before modification if it gets overwritten by
    /// this operation (when the chain overflows).
    ///
    /// Note: `error_code` is masked to the first 4 bits and `category_index` is masked to
    /// the first 3 bits.
    pub fn push_front(
        &mut self,
        error_code: ErrorCode,
        category_index: u8,
    ) -> Option<(ErrorCode, u8)> {
        // Get the last error code and formatter index in the chain,
        // if the formatter index is greater `0` that means the chain is full
        // and we return these from the function.
        let fmt_index_back = self.data & consts::FORMATTER_MASK[ERROR_CHAIN_LEN - 1];
        let result = if fmt_index_back > 0 {
            let ec_back = (self.data & consts::CODE_MASK[ERROR_CHAIN_LEN])
                >> (ERROR_CHAIN_LEN as u32 * consts::CODE_WIDTH);
            let fmt_index_back = fmt_index_back
                >> ((ERROR_CHAIN_LEN as u32 - 1) * consts::FORMATTER_IDX_WIDTH
                    + consts::FORMATTER_BITOFFSET);

            Some((ec_back as ErrorCode, (fmt_index_back - 1) as u8))
        } else {
            None
        };

        let fmt_indices = ((self.data & consts::ALL_FORMATTER_MASK) << consts::FORMATTER_IDX_WIDTH)
            | (consts::make_formatter_idx(category_index + 1) << consts::FORMATTER_BITOFFSET);

        let err_codes = ((self.data << consts::CODE_WIDTH) & consts::ALL_CODE_MASK)
            | consts::make_code(error_code);

        self.data = fmt_indices | err_codes;

        result
    }

    /// Chain this error with a new error specified by `error_code`.
    ///
    /// - `error_code`: The new error code that is set as the current one.
    /// - `category_index`: The index of the
    ///   [`ErrorCodeFormatter`](crate::ErrorCodeFormatter) in the slice returned by
    ///   [`T::chainable_category_formatters()`](ErrorCategory::chainable_category_formatters())
    ///   where `T` is the [`error category`](ErrorCategory) that the most recent error code
    ///   before this operation belongs to.
    ///
    /// This prepends the current error code to the front of the error chain and sets
    /// `error_code` as the new current error code.
    ///
    /// ### Panics
    /// If the feature `panic-on-overflow` is enabled and the error chain is already full
    /// before this operation, this function will panic. If the feature is not enabled and
    /// the error chain is already full, the last error in the chain will be lost.
    pub fn chain(&mut self, error_code: ErrorCode, category_index: u8) {
        // Returns the last error in the chain if it's full.
        let overflow = self.push_front(error_code, category_index);

        #[cfg(feature = "panic-on-overflow")]
        debug_assert!(
            overflow.is_none(),
            "chaining two errors overflowed; error chain is full"
        );
    }

    /// Iterate over the error chain.
    pub(crate) fn iter_chain(&self) -> ErrorDataChainIter {
        ErrorDataChainIter {
            error_codes: (self.data & consts::ALL_CODE_MASK) >> consts::CODE_WIDTH,
            formatters: (self.data & consts::ALL_FORMATTER_MASK) >> consts::FORMATTER_BITOFFSET,
        }
    }
}

/// An iterator over the error chain.
///
/// For every iteration a tuple is returned which contains:
/// - `0`: The error code at the current chain position.
/// - `1`: The formatter index of the next chain position if present.
pub(crate) struct ErrorDataChainIter {
    error_codes: u32,
    formatters: u32,
}

impl Iterator for ErrorDataChainIter {
    type Item = (ErrorCode, Option<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.formatters > 0 {
            let ec = self.error_codes & consts::CODE_MASK[0];
            self.error_codes >>= consts::CODE_WIDTH;
            self.formatters >>= consts::FORMATTER_IDX_WIDTH;

            let next_fmt_index = {
                let next_fmt_index = consts::make_formatter_idx(self.formatters as u8);
                if next_fmt_index > 0 {
                    Some(next_fmt_index as u8 - 1)
                } else {
                    None
                }
            };

            Some((ec as ErrorCode, next_fmt_index))
        } else {
            None
        }
    }
}
