//! Utilities used by the proc-macros.
//!
//! **This module has no stability guarantees.**

#[cfg(feature = "nightly")]
pub const fn const_assert(cond: bool, msg: &'static str) {
    assert!(cond, msg);
}

#[cfg(feature = "nightly")]
#[doc(hidden)]
#[macro_export]
macro_rules! const_assert {
    ($cond:expr, $msg:literal) => {
        const _: () = $crate::utils::const_assert($cond, $msg);
    };
}

#[cfg(not(feature = "nightly"))]
#[doc(hidden)]
#[macro_export]
macro_rules! const_assert {
    ($exp:expr, $msg:expr) => {
        #[deny(const_err)]
        #[allow(unused_must_use)]
        const _: () = {
            const ASSERT: bool = $exp;
            ASSERT as usize - 1usize;

            ()
        };
    };
}

#[cfg(feature = "std")]
mod types {
    pub use std::convert::From;
    pub use std::convert::Into;
    pub use std::fmt;
    pub use std::fmt::Debug;
    pub use std::mem;
    pub use std::result::Result;
}

#[cfg(not(feature = "std"))]
mod types {
    pub use core::convert::From;
    pub use core::convert::Into;
    pub use core::fmt;
    pub use core::fmt::Debug;
    pub use core::mem;
    pub use core::result::Result;
}

pub use types::*;
