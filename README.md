# embedded-error-chain

Easy error handling for embedded devices (`no_alloc` and `no_std`).

A rust library implementing easy error handling for embedded devices. An [`Error`] value is
only a single [`u32`] in size and supports up to 4 chained error codes. Each error code can
have a value from `0` to `15` (4 bits). All error codes come from an enum that implements
the [`ErrorCategory`] trait (a derive macro exists). This trait is also used to implement
debug printing and equality for each error code.

This library was inspired by libraries such as [error-chain](https://crates.io/crates/error-chain)
and [anyhow](https://crates.io/crates/anyhow), though its goal is to work in `no_std` and `no_alloc`
environments with very little memory overhead.

### Example
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

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(GyroAccError))]
#[repr(u8)]
enum CalibrationError {
    Inner,
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

fn calibrate() -> Result<(), Error<CalibrationError>> {
    gyro_acc_init().chain_err(CalibrationError::Inner)?;
    Ok(())
}
```

License: MIT
