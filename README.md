# embedded-error-chain

[![build](https://github.com/N3xed/embedded-error-chain/workflows/CI/badge.svg)](https://github.com/N3xed/embedded-error-chain/actions)
[![crates.io](https://img.shields.io/crates/v/embedded-error-chain.svg)](https://crates.io/crates/embedded-error-chain)
[![docs](https://docs.rs/embedded-error-chain/badge.svg)](https://docs.rs/embedded-error-chain/)

Easy error handling for embedded devices (no `liballoc` and `no_std`).

Errors are represented by error codes and come from enums that implement the
`ErrorCategory` trait (a derive macro exists), which is used for custom debug
printing per error code among other things. Each error code can have a value from `0`
to `15` (4 bits) and you can chain an error with up to four different error codes of
different categories.

The `Error` type encapsulates an error code and error chain, and is only a single
`u32` in size. There is also an untyped `DynError` type, which unlike `Error`
does not have a type parameter for the current error code. Its size is a `u32` +
pointer (`usize`), which can be used to forward source errors of different categories
to the caller.

This library was inspired by libraries such as
[error-chain](https://crates.io/crates/error-chain),
[anyhow](https://crates.io/crates/anyhow) and
[thiserror](https://crates.io/crates/thiserror), though it was made to work in `no_std`
**and** no `liballoc` environments with very little memory overhead.

## Example
```rust
use embedded_error_chain::prelude::*;

#[derive(Clone, Copy, ErrorCategory)]
#[repr(u8)]
enum SpiError {
    BusError,
    // ...
}

static LAST_GYRO_ACC_READOUT: usize = 200;

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(SpiError))]
#[repr(u8)]
enum GyroAccError {
    InitFailed,

    #[error("{variant} (readout={})", LAST_GYRO_ACC_READOUT)]
    ReadoutFailed,

    /// Value must be in range [0, 256)
    #[error("{variant}: {summary}")]
    InvalidValue,
}

fn main() {
    if let Err(err) = calibrate() {
        // log the error
        println!("{:?}", err);
        // ...
    }

    let readout = match gyro_acc_readout() {
        Ok(val) => val,
        Err(err) => {
            if let Some(spi_error) = err.code_of_category::<SpiError>() {
                // try to fix it
                0
            }
            else {
                panic!("unfixable spi error");
            }
        }
    };
}

fn spi_init() -> Result<(), SpiError> {
    Err(SpiError::BusError)
}

fn gyro_acc_init() -> Result<(), Error<GyroAccError>> {
    spi_init().chain_err(GyroAccError::InitFailed)?;
    Ok(())
}

fn gyro_acc_readout() -> Result<u32, Error<GyroAccError>> {
    Err(SpiError::BusError.chain(GyroAccError::InvalidValue))
}

fn calibrate() -> Result<(), DynError> {
    gyro_acc_init()?;
    // other stuff...
    Ok(())
}
```

License: MIT
