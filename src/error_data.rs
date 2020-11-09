use super::ErrorCode;

/// The maximum amount of error codes that can be chained to an [`Error`](super::Error).
///
/// This is equal to the amount of [`ErrorData::chain()`] (or
/// [`ChainError::chain()`](super::ChainError::chain()),
/// [`ResultExt::chain_err()`](super::ResultExt::chain_err())) you can make before the
/// chain overflows, and it either panics (if the feature `panic-on-overflow` is enabled)
/// or the oldest error code gets lost.
pub const ERROR_CHAIN_LEN: usize = 4;
/// The entire data of the error and its error code chain.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorData {
    /// Contains the entire data of the error and its error code chain.
    ///
    /// - Bits `b0..b20` contain 5 error codes, each error code is 4 bits.
    ///   - `b0..b4`: the error code of error (returned by [`Self::code()`])
    ///   - `b4..b8`: chained error 0
    ///   - `b8..b12`: chained error 1
    ///   - `b12..b16`: chained error 2
    ///   - `b16..b20`: chained error 3
    /// - Bits `b20..b32` contain 4 category indices, each index has 3 bits.
    ///   - `b20..b23`: formatter `index + 1` of chained error 0 (`0` means not present)
    ///                 (returned by [`Self::first_formatter_index()`])
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
    /// Create new `ErrorData` that conains the supplied `error_code` and has an empty chain.
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

    /// Get the error code of the error.
    #[inline(always)]
    pub fn code(&self) -> ErrorCode {
        (self.data & consts::CODE_MASK[0]) as ErrorCode
    }

    /// Get the first formatter index in the chain if available.
    #[inline(always)]
    pub fn first_formatter_index(&self) -> Option<u8> {
        let fmt_index =
            ((self.data & consts::FORMATTER_MASK[0]) >> consts::FORMATTER_BITOFFSET) as u8;
        if fmt_index > 0 {
            Some(fmt_index - 1)
        } else {
            None
        }
    }

    /// The number of chained error codes (does not include the error code representing
    /// this error).
    #[inline]
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

    /// Wether the error chain is full.
    #[inline]
    pub fn chain_full(&self) -> bool {
        self.chain_len() == ERROR_CHAIN_LEN
    }

    /// Push onto the front of the error chain, return the back of the chain before and if
    /// it gets overwritten (when the chain overflows).
    ///
    /// Note: `error_code` is masked to the first 4 bits and `category_index` is masked to
    /// the first 3 bits.
    #[inline(always)]
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

    /// Chain this error with a new error specified by `error_code` and `category_index`.
    ///
    /// TODO: document
    pub fn chain(&mut self, error_code: ErrorCode, category_index: u8) {
        // Returns the last error in the chain if it's full.
        let overflow = self.push_front(error_code, category_index);

        #[cfg(feature = "panic-on-overflow")]
        debug_assert!(
            overflow.is_none(),
            "chaining two errors overflowed; error chain is full"
        );
    }

    /// Iterate over the error chain (excluding the error code returned by [`ErrorData::code()`]).
    pub(crate) fn iter_chain(&self) -> ErrorDataChainIter {
        ErrorDataChainIter {
            error_codes: (self.data & consts::ALL_CODE_MASK) >> consts::CODE_WIDTH,
            formatters: (self.data & consts::ALL_FORMATTER_MASK) >> consts::FORMATTER_BITOFFSET,
        }
    }
}

/// An iterator over the error chain (excluding the error code returned by [`ErrorData::code()`]).
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
