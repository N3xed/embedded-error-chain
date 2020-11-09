use super::{
    error_category::{self, ErrorCodeFormatter},
    error_data::ErrorDataChainIter,
    marker, ErrorCategory, ErrorCategoryHandle, ErrorCode, ErrorData, ERROR_CHAIN_LEN,
};
use core::marker::PhantomData;
use core::{
    fmt::{self, Debug, Formatter},
    iter::FusedIterator,
};

#[repr(transparent)]
pub struct Error<C>(ErrorData, PhantomData<C>);

impl<C: ErrorCategory> Error<C> {
    /// Create a new [`Error`] with an empty chain from the supplied `error_code`.
    pub fn new(error_code: C) -> Error<C> {
        Error(ErrorData::new(error_code.into()), PhantomData)
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

        for (ec, handle) in self.iter() {
            if handle.is_same_category(&category_handle) && ec == error_code {
                return true;
            }
        }
        false
    }

    /// Query the error code contained in this error that belongs to the [`ErrorCategory`]
    /// `T`. Return `None` if this error was not caused by the specified error category.
    pub fn code_of_category<T: ErrorCategory>(&self) -> Option<T> {
        let category_handle = ErrorCategoryHandle::new::<T>();
        for (ec, handle) in self.iter() {
            if handle.is_same_category(&category_handle) {
                return Some(ec.into());
            }
        }
        None
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
    formatter_func: Option<ErrorCodeFormatter>,
    curr_error_code: ErrorCode,
    next_formatter_index: Option<u8>,
    chain_iter: ErrorDataChainIter,
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Error message example:
        // > ControlTaskError(0): init failed
        // > - ICM20689Error(0): init failed
        // > - SpiError(0): bus error

        let ec = self.0.code();
        let err: C = ec.into();
        writeln!(f, "{}({}): {:?}", C::NAME, ec, err)?;

        let mut formatter = error_category::get_next_formatter::<C>(self.0.first_formatter_index());
        for (ec, next_fmt_index) in self.0.iter_chain() {
            formatter = if let Some(formatter_func) = formatter {
                let (_, next_formatter) = formatter_func(ec, next_fmt_index, Some(f));

                next_formatter?.map(|func_val| func_val.into())
            } else {
                break;
            };
        }
        Ok(())
    }
}

pub trait ChainError<I: ErrorCategory, O: ErrorCategory, TAG> {
    /// Chain this [`Error`] with the supplied `error_code`.
    fn chain(self, error_code: O) -> Error<O>;
}

pub trait ResultChainError<T, I: ErrorCategory, O: ErrorCategory, TAG> {
    fn chain_err(self, error_code: O) -> Result<T, Error<O>>;
}

macro_rules! impl_chain_error {
    ($([$t:ident, $idx:literal]),*) => {
        $(
            impl<C: ErrorCategory> ChainError<C::$t, C, marker::$t> for Error<C::$t> {
                #[inline]
                fn chain(self, error_code: C) -> Error<C> {
                    let mut data = self.0;
                    data.chain(error_code.into(), $idx);
                    Error(data, PhantomData)
                }
            }

            impl<T, C: ErrorCategory> ResultChainError<T, C::$t, C, marker::$t> for Result<T, Error<C::$t>> {
                #[inline]
                fn chain_err(self, error_code: C) -> Result<T, Error<C>> {
                    match self {
                        Err(err) => Err(err.chain(error_code)),
                        Ok(val) => Ok(val),
                    }
                }
            }
        )+
    };
}

impl_chain_error!([C0, 0], [C1, 1], [C2, 2], [C3, 3]);

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
