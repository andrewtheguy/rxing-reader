use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },
    #[error("invalid state: {message}")]
    InvalidState { message: String },
    #[error("not found: {message}")]
    NotFound { message: String },
    #[error("invalid format: {message}")]
    InvalidFormat { message: String },
    #[error("checksum failed: {message}")]
    Checksum { message: String },
}
