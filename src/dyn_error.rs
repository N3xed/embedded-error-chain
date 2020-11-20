use crate::{
    format_chained, ChainError, Error, ErrorCategory, ErrorCategoryHandle, ErrorCode,
    ErrorCodeFormatter, ErrorData, ErrorIter, ERROR_CHAIN_LEN,
};
use core::{fmt, ptr};

/// Untyped counterpart to [`Error`].
///
/// This struct functions mostly identical to [`Error`] though its contained error code
/// can be of any [`ErrorCategory`] and does not depend on a type parameter. To enable
/// this, this struct in addition to an [`ErrorData`] value also contains an
/// [`ErrorCodeFormatter`] function pointer belonging to the [`ErrorCategory`] of the most
/// recent error.
///
/// All the limitation to error chaining outlined in [`Error`]'s documentation also apply
/// to [`DynError`]. But because the [error category](ErrorCategory) type of this error is
/// not known, whether or not it is possible to chain a [`DynError`] using
/// [`chain()`](ChainError::chain()) or [`chain_err()`](crate::ResultChainError::chain_err())
/// with an different `error_code` cannot be checked at compile time. So instead of a compile
/// error it will cause a panic if unlinked [error categories](ErrorCategory) are chained.
///
/// ```
/// # use embedded_error_chain::prelude::*;
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum FooError {
///     Err
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[error_category(links(FooError))]
/// #[repr(u8)]
/// enum BarError {
///     Err
/// }
///
/// fn cause_dyn_error() -> DynError {
///     (FooError::Err).into()
/// }
///
/// // Chain example
/// fn do_chain() -> DynError {
///     cause_dyn_error().chain(BarError::Err).into()
/// }
/// # do_chain();
/// ```
///
/// This will panic:
/// ```should_panic
/// # use embedded_error_chain::prelude::*;
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum FooError {
///     Err
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum BarError {
///     Err
/// }
///
/// fn cause_dyn_error() -> DynError {
///     (FooError::Err).into()
/// }
///
/// fn do_chain() -> DynError {
///     cause_dyn_error().chain(BarError::Err).into()
/// }
///
/// # do_chain();
/// ```
///
/// Note also the `.into()` after chaining. This is required because chaining results in
/// an [`Error<BarError>`] value being returned. This is also true for
/// [`chain_err()`](crate::ResultChainError::chain_err()).
///
/// ```
/// # use embedded_error_chain::prelude::*;
/// # #[derive(Clone, Copy, ErrorCategory)]
/// # #[repr(u8)]
/// # enum FooError {
/// #     Err
/// # }
/// #
/// # #[derive(Clone, Copy, ErrorCategory)]
/// # #[error_category(links(FooError))]
/// # #[repr(u8)]
/// # enum BarError {
/// #     Err
/// # }
/// #
/// fn cause_dyn_error_result() -> Result<(), DynError> {
///     Err((FooError::Err).into())
/// }
///
/// fn do_chain() -> Result<(), DynError> {
///     cause_dyn_error_result().chain_err(BarError::Err)?;
///     Ok(())
/// }
/// # do_chain();
/// ```
/// Here we use the property of the `?` operator that automatically converts the
/// [`Error<BarError>`] to a [`DynError`] using `.into()`. This works because [`DynError`]
/// implements the [`From`] trait for any [`Error<T>`](Error).
///
/// The following would not work and won't compile:
/// ```compile_fail
/// # use embedded_error_chain::prelude::*;
/// # #[derive(Clone, Copy, ErrorCategory)]
/// # #[repr(u8)]
/// # enum FooError {
/// #     Err
/// # }
/// #
/// # #[derive(Clone, Copy, ErrorCategory)]
/// # #[error_category(links(FooError))]
/// # #[repr(u8)]
/// # enum BarError {
/// #     Err
/// # }
/// #
/// # fn cause_dyn_error_result() -> Result<(), DynError> {
/// #     Err((FooError::Err).into())
/// # }
/// #
/// fn do_chain() -> Result<(), DynError> {
///     cause_dyn_error_result().chain_err(BarError::Err)
/// }
/// # do_chain();
/// ```
///
#[derive(Clone)]
pub struct DynError {
    error: ErrorData,
    category_formatter: ErrorCodeFormatter,
}

impl PartialEq for DynError {
    fn eq(&self, other: &DynError) -> bool {
        self.error == other.error
            && ptr::eq(
                self.category_formatter as *const (),
                other.category_formatter as *const (),
            )
    }
}
impl Eq for DynError {}

impl DynError {
    /// Create a [`DynError`] from an `error_code` belonging to [error
    /// category](ErrorCategory) `C`.
    #[inline]
    pub fn new<C: ErrorCategory>(error_code: C) -> DynError {
        DynError {
            error: ErrorData::new(error_code.into()),
            category_formatter: format_chained::<C>,
        }
    }

    /// Create a [`DynError`] from its raw parts.
    #[inline]
    pub fn from_raw_parts(
        error_data: ErrorData,
        category_formatter: ErrorCodeFormatter,
    ) -> DynError {
        DynError {
            error: error_data,
            category_formatter,
        }
    }

    /// Turn this dynamic error into its raw parts.
    pub fn into_raw_parts(self) -> (ErrorData, ErrorCodeFormatter) {
        (self.error, self.category_formatter)
    }

    /// Get the error code of the most recent error.
    #[inline(always)]
    pub fn code(&self) -> ErrorCode {
        self.error.code()
    }

    /// Get the length of the error chain.
    #[inline(always)]
    pub fn chain_len(&self) -> usize {
        self.error.chain_len()
    }

    /// Get the capacity of the error chain.
    ///
    /// Always returns [`ERROR_CHAIN_LEN`].
    pub const fn chain_capacity(&self) -> usize {
        ERROR_CHAIN_LEN
    }

    /// Get the [`ErrorCategoryHandle`] of the most recent error.
    #[inline(always)]
    pub fn category_handle(&self) -> ErrorCategoryHandle {
        (self.category_formatter)(0, None, None).0
    }

    /// Get the [`ErrorCodeFormatter`] function of the most recent error.
    #[inline(always)]
    pub fn formatter(&self) -> ErrorCodeFormatter {
        self.category_formatter
    }

    /// Return `true` if the most recent error code belongs to the [error category](ErrorCategory) `C`.
    pub fn is<C: ErrorCategory>(&self) -> bool {
        self.category_handle().is_handle_of::<C>()
    }

    /// Try to convert this untyped dynamic error into a statically typed error.
    ///
    /// Succeeds and returns the equivalent [`Error`] of this [`DynError`] if
    /// [`self.is::<C>()`](Self::is()) returns `true`, otherwise returns an [`Err`]
    /// containing the original [`DynError`].
    pub fn try_into<C: ErrorCategory>(self) -> Result<crate::Error<C>, Self> {
        if self.is::<C>() {
            Ok(crate::Error::from_raw(self.error))
        } else {
            Err(self)
        }
    }

    /// Query if this error was caused by `error_code` which belongs to the [error
    /// category](ErrorCategory) `T`.
    pub fn caused_by<T: ErrorCategory>(&self, error_code: T) -> bool {
        let error_code: ErrorCode = error_code.into();
        let category_handle = ErrorCategoryHandle::new::<T>();

        self.iter()
            .any(|(ec, handle)| handle == category_handle && ec == error_code)
    }

    /// Query the error code contained in this error that belongs to the (error
    /// category)[`ErrorCategory`] `T`. Return `None` if this error was not caused by the
    /// specified error category.
    pub fn code_of_category<T: ErrorCategory>(&self) -> Option<T> {
        let category_handle = ErrorCategoryHandle::new::<T>();
        self.iter().find_map(|(ec, handle)| {
            if handle == category_handle {
                Some(ec.into())
            } else {
                None
            }
        })
    }

    /// Create an iterator that iterates over all error codes that caused this error.
    #[inline]
    pub fn iter(&self) -> ErrorIter {
        ErrorIter {
            formatter_func: Some(self.category_formatter),
            curr_error_code: self.error.code(),
            next_formatter_index: self.error.first_formatter_index(),
            chain_iter: self.error.iter_chain(),
        }
    }

    /// Try to chain this dynamically typed [`DynError`] with `error_code` of
    /// [error category](ErrorCategory) `C`.
    ///
    /// A call to this function only succeeds if the slice of [`ErrorCodeFormatter`]s
    /// returned by
    /// [`C::chainable_category_formatters()`](ErrorCategory::chainable_category_formatters())
    /// contains [`self.formatter()`](Self::formatter()).
    ///
    /// Note that this function has time complexity `O(n)` where `n` is the length of the
    /// slice returned by
    /// [`C::chainable_category_formatters()`](ErrorCategory::chainable_category_formatters()).
    pub fn try_chain<C: ErrorCategory>(self, error_code: C) -> Result<Error<C>, Self> {
        C::chainable_category_formatters()
            .iter()
            .enumerate()
            .find_map(|(i, formatter)| {
                if ptr::eq(
                    *formatter as *const (),
                    self.category_formatter as *const (),
                ) {
                    let mut data: ErrorData = self.error;
                    ErrorData::chain(&mut data, error_code.into(), i as u8);
                    Some(Error::from_raw(data))
                } else {
                    None
                }
            })
            .ok_or(self)
    }
}

impl<O: ErrorCategory> ChainError<O, DynError> for DynError {
    /// Chain a [`DynError`] with any error code of a linked [`ErrorCategory`].
    ///
    /// Note that this function has complexity `O(n)` where `n` is the length of the slice
    /// returned by
    /// [`O::chainable_category_formatters()`](ErrorCategory::chainable_category_formatters()).
    ///
    /// ### Panics
    /// A call to this function panics if the slice of [`ErrorCodeFormatter`]s
    /// returned by
    /// [`O::chainable_category_formatters()`](ErrorCategory::chainable_category_formatters())
    /// does **not** contain [`self.formatter()`](DynError::formatter()).
    fn chain(self, error_code: O) -> Error<O> {
        self.try_chain(error_code)
            .expect("cannot chain unlinked error categories")
    }
}

impl fmt::Debug for DynError {
    /// Debug format this error and its chain.
    ///
    /// Error message example:
    /// ```txt
    /// ControlTaskError(0): init failed
    /// - ICM20689Error(0): init failed
    /// - SpiError(0): bus error
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (_, fmt_result) =
            (self.category_formatter)(self.code(), self.error.first_formatter_index(), Some(f));

        let mut formatter_func = fmt_result?;
        for (ec, next_fmt_index) in self.error.iter_chain() {
            formatter_func = if let Some(formatter_func) = formatter_func {
                write!(f, "\n- ")?;
                let (_, next_formatter) = formatter_func.into()(ec, next_fmt_index, Some(f));

                next_formatter?
            } else {
                break;
            };
        }
        Ok(())
    }
}

impl<C: ErrorCategory> From<Error<C>> for DynError {
    #[inline]
    fn from(error: crate::Error<C>) -> Self {
        DynError::from_raw_parts(error.into(), format_chained::<C>)
    }
}

impl<C: ErrorCategory> From<C> for DynError {
    #[inline]
    fn from(error: C) -> Self {
        DynError::from_raw_parts(ErrorData::new(error.into()), format_chained::<C>)
    }
}
