use embedded_hal_async::i2c::ErrorKind;


#[derive(Debug)]
pub enum Error {
    EncodeError(prost::EncodeError),
    DecoderError(prost::DecodeError),
    IoError(std::io::Error),

    UnexpectedEndOfStream,
    Timeout,
    Protocol(super::messages::Status),
    I2C(ErrorKind),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<prost::EncodeError> for Error {
    fn from(e: prost::EncodeError) -> Self {
        Error::EncodeError(e)
    }
}

impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Self {
        Error::DecoderError(e)
    }
}

impl embedded_hal_async::i2c::Error for Error {
    fn kind(&self) -> ErrorKind {
        if let Error::I2C(k) = self {
            *k
        } else {
            ErrorKind::Other
        }
    }
}