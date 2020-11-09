use super::ErrorCode;
use core::fmt::{self, Debug, Formatter};

/// A chained formatter function.
///
/// A single `ErrorCodeFormatter` function is considered to be uniquely associated with a
/// type that implements [`ErrorCategory`]. Meaning one such function only ever returns the
/// [`ErrorCategoryHandle`] for that associated [`ErrorCategory`], and never for another.
///
/// This function serves multiple purposes:
/// 1. If `f` is [`Some`] then this functions formats `error_code` using `f` with
///   the following format:  
///    `- {ErrorCategory::NAME}({error_code}): {debug fmt of error_code}`
/// 2. If `next_formatter` is `Some(index)` then it returns the chained formatter of the
///    associated [`ErrorCategory`] indexed by `index`. A `Some(`[`ErrorCodeFormatterVal`]`)` is
///    returned if `index` is within bounds of the chainable categories (see
///    [`ErrorCategory::chainable_category_formatters()`]).
/// 3. This function addtionally always returns a [`ErrorCategoryHandle`] that represents
///    the associated [`ErrorCategory`].
pub type ErrorCodeFormatter = fn(
    error_code: ErrorCode,
    next_formatter: Option<u8>,
    f: Option<&mut Formatter<'_>>,
) -> (
    ErrorCategoryHandle,
    Result<Option<ErrorCodeFormatterVal>, fmt::Error>,
);

/// A wrapped [`ErrorCodeFormatter`] value.
///
/// This is returned from the [`ErrorCodeFormatter`] function itself as a workaround,
/// because function type definitions cannot reference themselves. The contained function
/// is actually also a [`ErrorCodeFormatter`] value.
#[repr(transparent)]
pub struct ErrorCodeFormatterVal(ErrorCodeFormatter);

impl ErrorCodeFormatterVal {
    /// Create a new wrapped [`ErrorCodeFormatter`] value from `func`.
    pub fn new(func: ErrorCodeFormatter) -> ErrorCodeFormatterVal {
        ErrorCodeFormatterVal(func)
    }

    /// Unwrap the wrapped [`ErrorCodeFormatter`] value.
    pub fn into(self) -> ErrorCodeFormatter {
        self.0
    }
}
pub trait ErrorCategory: Copy + Into<ErrorCode> + From<ErrorCode> + Debug {
    const NAME: &'static str;

    type C0: ErrorCategory;
    type C1: ErrorCategory;
    type C2: ErrorCategory;
    type C3: ErrorCategory;

    fn chainable_category_formatters() -> &'static [ErrorCodeFormatter] {
        &[
            format_chained::<Self::C0>,
            format_chained::<Self::C1>,
            format_chained::<Self::C2>,
            format_chained::<Self::C3>,
        ]
    }
}

/// A handle to a type that implements [`ErrorCategory`].
#[derive(Debug, PartialEq, Eq)]
pub struct ErrorCategoryHandle {
    type_id: usize,
    name: &'static str,
}

impl ErrorCategoryHandle {
    /// Create a new handle from the type parameter `C`.
    pub fn new<C: ErrorCategory>() -> ErrorCategoryHandle {
        Self {
            name: C::NAME,
            type_id: C::chainable_category_formatters as *const () as usize,
        }
    }

    /// Get the name of this associated [`ErrorCategory`].
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Check wether two `ErrorCategoryHandle`s are handles of the same [`ErrorCategory`].
    pub fn is_same_category(&self, other: &ErrorCategoryHandle) -> bool {
        self.type_id == other.type_id
    }
}

/// Get the next formatter of a chained error with `next_formatter`.
#[inline(always)]
pub fn get_next_formatter<C: ErrorCategory>(
    next_formatter_index: Option<u8>,
) -> Option<ErrorCodeFormatter> {
    if let Some(idx) = next_formatter_index {
        let formatters = C::chainable_category_formatters();

        let idx = idx as usize;
        if idx < formatters.len() {
            return Some(formatters[idx]);
        }
    }
    None
}

/// Debug format the given `error_code` using `f` (if `f` is `Some`), get the
/// [`ErrorCategoryHandle`] of the type parameter `C`, and get the next [`ErrorCodeFormatter`]
/// if `next_formatter` is `Some`.
pub fn format_chained<C: ErrorCategory>(
    error_code: ErrorCode,
    next_formatter: Option<u8>,
    f: Option<&mut Formatter<'_>>,
) -> (
    ErrorCategoryHandle,
    Result<Option<ErrorCodeFormatterVal>, fmt::Error>,
) {
    let fmt_res = if let Some(f) = f {
        let err: C = error_code.into();
        writeln!(f, "- {}({}): {:?}", C::NAME, error_code, err)
    } else {
        Ok(())
    };

    (
        ErrorCategoryHandle::new::<C>(),
        fmt_res.map(|_| {
            let func = get_next_formatter::<C>(next_formatter);
            func.map(ErrorCodeFormatterVal::new)
        }),
    )
}

/// This marker type is used for any [`ErrorCategory::C0`] to [`C3`](ErrorCategory::C3)
/// which is unused.
#[derive(Debug, Clone, Copy)]
pub enum Unused {}

impl ErrorCategory for Unused {
    const NAME: &'static str = "";
    type C0 = Unused;
    type C1 = Unused;
    type C2 = Unused;
    type C3 = Unused;

    fn chainable_category_formatters() -> &'static [ErrorCodeFormatter] {
        &[]
    }
}

impl From<ErrorCode> for Unused {
    fn from(_: ErrorCode) -> Self {
        unreachable!()
    }
}

impl Into<ErrorCode> for Unused {
    fn into(self) -> ErrorCode {
        match self {}
    }
}
