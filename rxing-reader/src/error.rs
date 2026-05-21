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

impl Error {
    pub(crate) fn invalid_argument(message: impl Into<ErrorMessage>) -> Self {
        Self::InvalidArgument {
            message: message.into(),
        }
    }

    pub(crate) fn invalid_state(message: impl Into<ErrorMessage>) -> Self {
        Self::InvalidState {
            message: message.into(),
        }
    }

    pub(crate) fn not_found(message: impl Into<ErrorMessage>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub(crate) fn invalid_format(message: impl Into<ErrorMessage>) -> Self {
        Self::InvalidFormat {
            message: message.into(),
        }
    }

    pub(crate) fn checksum(message: impl Into<ErrorMessage>) -> Self {
        Self::Checksum {
            message: message.into(),
        }
    }
}
