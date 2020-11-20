use crate::ErrorCode;

use core::{
    fmt::{self, Debug, Formatter},
    ptr,
};

/// A chained formatter function for a single error category.
///
/// A single `ErrorCodeFormatter` function is considered to be uniquely associated with a
/// type that implements [`ErrorCategory`]. Meaning one such function only ever returns the
/// [`ErrorCategoryHandle`] for that associated [`ErrorCategory`], and never for another.
///
/// This function serves multiple purposes:
/// 1. If `f` is [`Some`] then this functions formats `error_code` using `f`.
/// 2. If `next_formatter` is `Some(index)` then it returns the chained formatter of the
///    associated [`ErrorCategory`] indexed by `index`. A `Some(`[`ErrorCodeFormatterVal`]`)` is
///    returned if `index` is within bounds of the chainable categories (see
///    [`ErrorCategory::chainable_category_formatters()`]).
/// 3. This function additionally always returns a [`ErrorCategoryHandle`] that represents
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

/// A trait that implements the logic for debug printing and [`ErrorCode`] conversion. It
/// also specifies the links to other error categories that allows errors of
/// different categories to be chained.
///
/// Note: Only up to 6 linked error categories are supported.
///
/// See [`Error`](crate::Error), [`DynError`](crate::DynError) and
/// [`ErrorData`](crate::ErrorData) for more information.
pub trait ErrorCategory: Copy + Into<ErrorCode> + From<ErrorCode> + Debug {
    /// The text name of this category used for formatting.
    const NAME: &'static str;

    /// Type of linked error category 0.
    ///
    /// Set to [`Unused`] if unused.
    type L0: ErrorCategory;
    /// Type of linked error category 1.
    ///
    /// Set to [`Unused`] if unused.
    type L1: ErrorCategory;
    /// Type of linked error category 2.
    ///
    /// Set to [`Unused`] if unused.
    type L2: ErrorCategory;
    /// Type of linked error category 3.
    ///
    /// Set to [`Unused`] if unused.
    type L3: ErrorCategory;
    /// Type of linked error category 4.
    ///
    /// Set to [`Unused`] if unused.
    type L4: ErrorCategory;
    /// Type of linked error category 5.
    ///
    /// Set to [`Unused`] if unused.
    type L5: ErrorCategory;

    /// Get a slice of all [`ErrorCodeFormatter`] functions for all [error
    /// categories](ErrorCategory) that this [error category](ErrorCategory) is linked to.
    ///
    /// Specifically returns a slice of function pointers to the error code formatter
    /// function of [`Self::L0`] up to [`Self::L5`]. Each element in the returned slice
    /// corresponds to the formatter function of the [error category](ErrorCategory) type
    /// `Self::Lx` where `x` is the index of the element. The slice can be smaller than 6
    /// elements, if the excluded linked error categories are unused (i.e. `Self::Lx` is
    /// set to [`Unused`]).
    ///
    /// All formatter functions contained in the returned slice must have identical
    /// behavior to [`format_chained()`] with the exception that the formatting can
    /// differ.
    fn chainable_category_formatters() -> &'static [ErrorCodeFormatter] {
        &[
            format_chained::<Self::L0>,
            format_chained::<Self::L1>,
            format_chained::<Self::L2>,
            format_chained::<Self::L3>,
            format_chained::<Self::L4>,
            format_chained::<Self::L5>,
        ]
    }
}

/// A handle to a type that implements [`ErrorCategory`].
#[derive(Debug)]
pub struct ErrorCategoryHandle {
    name: &'static str,
    chainable_category_formatters: fn() -> &'static [ErrorCodeFormatter],
}

impl ErrorCategoryHandle {
    /// Create a new handle from the type parameter `C`.
    pub fn new<C: ErrorCategory>() -> ErrorCategoryHandle {
        Self {
            name: C::NAME,
            chainable_category_formatters: C::chainable_category_formatters,
        }
    }

    /// Get the name of this associated [`ErrorCategory`].
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Check whether this handle is a handle of the [`ErrorCategory`] `C`.
    #[inline]
    pub fn is_handle_of<C: ErrorCategory>(&self) -> bool {
        ptr::eq(self.name.as_ptr(), C::NAME.as_ptr())
            && ptr::eq(
                self.chainable_category_formatters as *const (),
                C::chainable_category_formatters as *const (),
            )
    }
}

impl PartialEq for ErrorCategoryHandle {
    fn eq(&self, other: &ErrorCategoryHandle) -> bool {
        ptr::eq(self.name.as_ptr(), other.name.as_ptr())
            && ptr::eq(
                self.chainable_category_formatters as *const (),
                other.chainable_category_formatters as *const (),
            )
    }
}
impl Eq for ErrorCategoryHandle {}

/// Debug format the given `error_code` using `f` if `f` is `Some`, get the
/// [`ErrorCategoryHandle`] of the type parameter `C`, and get the next [`ErrorCodeFormatter`]
/// if `next_formatter` is `Some`.
///
/// If `f` is `Some()` the following format is used:  
///    `{C::NAME}({error_code}): {<error_code as C>:?}`
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
        write!(f, "{}({}): {:?}", C::NAME, error_code, err)
    } else {
        Ok(())
    };

    (
        ErrorCategoryHandle::new::<C>(),
        fmt_res.map(|_| {
            // Get the next formatter function if `next_formatter` is `Some`.
            next_formatter.and_then(|idx| {
                let idx = idx as usize;
                let formatters = C::chainable_category_formatters();

                if idx < formatters.len() {
                    Some(ErrorCodeFormatterVal::new(formatters[idx]))
                } else {
                    None
                }
            })
        }),
    )
}

/// This marker type is used for any [`ErrorCategory::L0`] to [`ErrorCategory::L5`]
/// which is unused.
#[derive(Debug, Clone, Copy)]
pub enum Unused {}

impl ErrorCategory for Unused {
    const NAME: &'static str = "";
    type L0 = Unused;
    type L1 = Unused;
    type L2 = Unused;
    type L3 = Unused;
    type L4 = Unused;
    type L5 = Unused;

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
