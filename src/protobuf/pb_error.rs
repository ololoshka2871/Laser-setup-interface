#[derive(Debug)]
pub enum Error {
    EncodeError(prost::EncodeError),
    DecoderError(prost::DecodeError),
    IoError(std::io::Error),

    UnexpectedEndOfStream,
    Timeout,
    Protocol(super::messages::Status),
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
