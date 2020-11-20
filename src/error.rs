use crate::{
    error_category::{self, ErrorCodeFormatter},
    error_data::ErrorDataChainIter,
    marker, DynError, ErrorCategory, ErrorCategoryHandle, ErrorCode, ErrorData, ERROR_CHAIN_LEN,
};
use core::marker::PhantomData;
use core::{
    fmt::{self, Debug, Formatter},
    iter::FusedIterator,
};

/// A typed error with an optional error chain of up to four source errors that represent
/// the cause of this error.
///
/// The error chain is a singly linked list of most recent to oldest source error with a
/// maximum length of 4. When chaining two errors with [`chain()`](ChainError::chain()) or
/// [`chain_err()`](ResultChainError::chain_err()) the error code of the current error is
/// prepended to the front of the linked list. If the linked list is already at its
/// maximum length before chaining, and the feature `panic-on-overflow` is enabled, the
/// chaining operation will panic, otherwise the oldest error will be lost. After the
/// current error code has been prepended, the new error code will be set as the current
/// and the chain operation will return a new [`Error`] typed with the [`ErrorCategory`]
/// of the new error code.
///
/// Chaining an [`Error`] with a new error code is only possible if the [`ErrorCategory`]
/// of the new error code links to the [`ErrorCategory`] of the current error code. This
/// is done using the `links` argument in the `error_category` attribute, if
/// [`ErrorCategory`] is implemented using the derive macro.
///
/// ```
/// # use embedded_error_chain::prelude::*;
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum CurrentError {
///     Err
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[error_category(links(CurrentError))]
/// //               ~~~~~~~~~~~~~~~~~~~
/// // This allows `CurrentError` to be chained with `NewError`
/// #[repr(u8)]
/// enum NewError {
///     Err
/// }
///
/// // Chain example
/// fn do_chain() -> Error<NewError> {
///     (CurrentError::Err).chain(NewError::Err)
/// }
/// # do_chain();
/// ```
///
/// This does not compile:
/// ```compile_fail
/// # use embedded_error_chain::prelude::*;
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum CurrentError {
///     Err
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum NewError {
///     Err
/// }
///
/// // Chain example
/// fn do_chain() -> Error<NewError> {
///     (CurrentError::Err).chain(NewError::Err)
/// }
/// # do_chain();
/// ```
///
/// Unlike [`DynError`](crate::DynError) which does not have a type parameter, [`Error`]'s
/// type parameter specifies the [`ErrorCategory`] of the most recent error (also called
/// current error). This allows the size of this struct to be reduced and so the struct is
/// guaranteed to only be one [`u32`] or 4 bytes in size (the same size as [`ErrorData`]),
/// whereas [`DynError`](crate::DynError) contains an additional pointer ([`usize`]).
///
/// Additionally because the [error category](`ErrorCategory`) of the first error is known at
/// compile time, this allows for the [`chain()`](ChainError::chain()) and
/// [`chain_err()`](ResultChainError::chain_err()) operations to be error checked at
/// compile time and for them to have a constant execution time `O(1)`.
///
/// But a consequence of this type parameter is, that when returning an [`Error`] from a
/// function that has to forward multiple errors of *different* [error
/// categories](ErrorCategory), you must always [`chain()`](ChainError::chain()) or
/// [`chain_err()`](ResultChainError::chain_err()) them. So instead of an inner error
/// being directly forwarded to the caller, you must have a single error of indirection in
/// between.
///
/// ```
/// # use embedded_error_chain::prelude::*;
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[repr(u8)]
/// enum InnerError {
///     SomeError
/// }
///
/// #[derive(Clone, Copy, ErrorCategory)]
/// #[error_category(links(InnerError))]
/// #[repr(u8)]
/// enum OuterError {
///     Inner
/// }
///
/// # fn causes_inner_error() -> Result<(), InnerError> { Err(InnerError::SomeError) }
/// #
/// fn do_something_that_may_error() -> Result<(), Error<OuterError>> {
///     causes_inner_error().chain_err(OuterError::Inner)?;
///     // other stuff that causes `OuterError`...
/// #   Ok(())
/// }
/// # do_something_that_may_error();
/// ```
///
/// If you want to directly forward a single or multiple source errors with different
/// unrelated [error categories](ErrorCategory) and you don't need the advantages outlined
/// above use [`DynError`] instead.
#[repr(transparent)]
pub struct Error<C>(ErrorData, PhantomData<C>);

impl<C> Error<C> {
    /// Create a new [`Error`] with an empty chain from the supplied raw `error_code`.
    ///
    /// This function is memory-safe and will never panic, but if `error_code` is not part the
    /// [`ErrorCategory`] `C` the behavior of all method calls on the returned [`Error`]
    /// is undefined.
    #[inline(always)]
    pub const fn new_raw(error_code: ErrorCode) -> Error<C> {
        Error(ErrorData::new(error_code), PhantomData)
    }

    /// Crate a new [`Error`] from raw [`ErrorData`].
    ///
    /// This function is memory-safe and will never panic, but if `error_data.code()` is
    /// not part the [`ErrorCategory`] `C` or the contained error chain is invalid, the
    /// behavior of all method calls on the returned [`Error`] is undefined.
    pub const fn from_raw(error_data: ErrorData) -> Error<C> {
        Error(error_data, PhantomData)
    }
}

impl<C> Error<C> {
    /// Get the capacity of the error chain.
    ///
    /// Always returns [`ERROR_CHAIN_LEN`].
    pub const fn chain_capacity(&self) -> usize {
        ERROR_CHAIN_LEN
    }
}

impl<C: ErrorCategory> Error<C> {
    /// Create a new [`Error`] with an empty chain from the supplied `error_code`.
    #[inline(always)]
    pub fn new(error_code: C) -> Error<C> {
        Error(ErrorData::new(error_code.into()), PhantomData)
    }

    /// Get the error code of the latest error.
    #[inline]
    pub fn code(&self) -> C {
        self.0.code().into()
    }

    /// Get the length of the error chain.
    pub fn chain_len(&self) -> usize {
        self.0.chain_len()
    }

    /// Query if this error was caused by `error_code` which belongs to the error category
    /// `T`.
    pub fn caused_by<T: ErrorCategory>(&self, error_code: T) -> bool {
        let error_code: ErrorCode = error_code.into();
        let category_handle = ErrorCategoryHandle::new::<T>();

        self.iter()
            .any(|(ec, handle)| handle == category_handle && ec == error_code)
    }

    /// Query the error code contained in this error that belongs to the [`ErrorCategory`]
    /// `T`. Return `None` if this error was not caused by the specified error category.
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

    /// Create an iterator that iterates over all error codes in this error.
    pub fn iter(&self) -> ErrorIter {
        ErrorIter {
            formatter_func: Some(error_category::format_chained::<C>),
            curr_error_code: self.0.code(),
            next_formatter_index: self.0.first_formatter_index(),
            chain_iter: self.0.iter_chain(),
        }
    }
}

/// An iterator over all error codes in this [`Error`].
///
/// Returns a tuple with the following items:
/// - `0`: The [`ErrorCode`] of this error.
/// - `1`: A [`ErrorCategoryHandle`] to the [`ErrorCategory`](super::ErrorCategory) of
///   this error.
pub struct ErrorIter {
    pub(crate) formatter_func: Option<ErrorCodeFormatter>,
    pub(crate) curr_error_code: ErrorCode,
    pub(crate) next_formatter_index: Option<u8>,
    pub(crate) chain_iter: ErrorDataChainIter,
}

impl Iterator for ErrorIter {
    type Item = (ErrorCode, ErrorCategoryHandle);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(formatter_func) = self.formatter_func {
            let (err_cat_handle, next_formatter_res) =
                formatter_func(0, self.next_formatter_index.take(), None);
            let error_code = self.curr_error_code;

            if let (Some((next_error_code, next_next_formatter_index)), Ok(Some(next_formatter))) =
                (self.chain_iter.next(), next_formatter_res)
            {
                self.curr_error_code = next_error_code;
                self.next_formatter_index = next_next_formatter_index;
                self.formatter_func = Some(next_formatter.into());
            } else {
                self.formatter_func = None;
            }

            Some((error_code, err_cat_handle))
        } else {
            None
        }
    }
}
impl FusedIterator for ErrorIter {}

impl<C: ErrorCategory> Debug for Error<C> {
    /// Debug format this error and its chain.
    ///
    /// Delegates to [`DynError::fmt()`].
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        DynError::from(*self).fmt(f)
    }
}

/// A trait that allows chaining of [`Error`] and [`DynError`](crate::DynError) values and
/// any value of a type that implements [`ErrorCategory`].
pub trait ChainError<O: ErrorCategory, Tag> {
    /// Chain this error with the supplied `error_code`.
    ///
    /// ### Panics
    /// If the [error category](ErrorCategory) `O` is not linked with the [`ErrorCategory`]
    /// of the most recent error code, this function will panic.
    fn chain(self, error_code: O) -> Error<O>;
}

/// A trait that allows chaining if a [`Result`] contains an [`Error`] value.
pub trait ResultChainError<T, O: ErrorCategory, Tag> {
    /// If the results contains an [`Err`] value, chain it with the supplied `error_code`
    /// and return [`Err`] with the result, otherwise forward the [`Ok`] value.
    ///
    /// ### Panics
    /// If this [`Result`] is an [`Err`] value and the [error category](ErrorCategory) `O`
    /// is not linked with the [`ErrorCategory`] of the most recent error code in the
    /// error, this function will panic.
    fn chain_err(self, error_code: O) -> Result<T, Error<O>>;
}

macro_rules! impl_chain_error {
    ($([$t:ident, $idx:literal]),*) => {
        $(
            impl<C: ErrorCategory> ChainError<C, (marker::$t, marker::Error_t)> for Error<C::$t> {
                #[inline(always)]
                fn chain(self, error_code: C) -> Error<C> {
                    let mut data: ErrorData = self.0;
                    ErrorData::chain(&mut data, error_code.into(), $idx);
                    Error(data, PhantomData)
                }
            }

            impl<C: ErrorCategory> ChainError<C, (marker::$t, marker::Concrete_t)> for C::$t {
                #[inline(always)]
                fn chain(self, error_code: C) -> Error<C> {
                    Error::new(self).chain(error_code)
                }
            }
        )+
    };
}

impl_chain_error!([L0, 0], [L1, 1], [L2, 2], [L3, 3], [L4, 4], [L5, 5]);

impl<OK, ERR, O, TAG> ResultChainError<OK, O, TAG> for Result<OK, ERR>
where
    O: ErrorCategory,
    ERR: ChainError<O, TAG>,
{
    #[inline]
    fn chain_err(self, error_code: O) -> Result<OK, Error<O>> {
        match self {
            Err(err) => Err(err.chain(error_code)),
            Ok(val) => Ok(val),
        }
    }
}

impl<C: ErrorCategory> PartialEq for Error<C> {
    fn eq(&self, other: &Error<C>) -> bool {
        self.0 == other.0
    }
}
impl<C: ErrorCategory> Eq for Error<C> {}

impl<C: ErrorCategory> Clone for Error<C> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Error(self.0, PhantomData)
    }

    #[inline(always)]
    fn clone_from(&mut self, source: &Self) {
        self.0 = source.0;
    }
}

impl<C: ErrorCategory> Copy for Error<C> {}

impl<C: ErrorCategory> From<C> for Error<C> {
    #[inline(always)]
    fn from(error_code: C) -> Self {
        Error::new(error_code)
    }
}

impl<C: ErrorCategory> From<Error<C>> for ErrorData {
    #[inline(always)]
    fn from(error: Error<C>) -> Self {
        error.0
    }
}
