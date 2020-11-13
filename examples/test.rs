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

    let _readout = match gyro_acc_readout() {
        Ok(val) => val,
        Err(err) => {
            if let Some(_spi_error) = err.code_of_category::<SpiError>() {
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