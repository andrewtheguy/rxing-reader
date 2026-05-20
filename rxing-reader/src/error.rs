use std::borrow::Cow;

use thiserror::Error;

type ErrorMessage = Cow<'static, str>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid argument: {message}")]
    InvalidArgument { message: ErrorMessage },
    #[error("invalid state: {message}")]
    InvalidState { message: ErrorMessage },
    #[error("not found: {message}")]
    NotFound { message: ErrorMessage },
    #[error("invalid format: {message}")]
    InvalidFormat { message: ErrorMessage },
    #[error("checksum failed: {message}")]
    Checksum { message: ErrorMessage },
}
