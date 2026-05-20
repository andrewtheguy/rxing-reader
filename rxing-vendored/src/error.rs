use thiserror::Error as ThisError;

#[derive(ThisError, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    #[error("invalid argument")]
    InvalidArgument,
    #[error("invalid argument: {0}")]
    InvalidArgumentMessage(String),
    #[error("unsupported operation")]
    UnsupportedOperation,
    #[error("unsupported operation: {0}")]
    UnsupportedOperationMessage(String),
    #[error("invalid state")]
    InvalidState,
    #[error("invalid state: {0}")]
    InvalidStateMessage(String),
    #[error("arithmetic error")]
    Arithmetic,
    #[error("arithmetic error: {0}")]
    ArithmeticMessage(String),
    #[error("not found")]
    NotFound,
    #[error("not found: {0}")]
    NotFoundMessage(String),
    #[error("invalid format")]
    InvalidFormat,
    #[error("invalid format: {0}")]
    InvalidFormatMessage(String),
    #[error("checksum failed")]
    Checksum,
    #[error("checksum failed: {0}")]
    ChecksumMessage(String),
    #[error("reader error")]
    Reader,
    #[error("reader error: {0}")]
    ReaderMessage(String),
    #[error("writer error")]
    Writer,
    #[error("writer error: {0}")]
    WriterMessage(String),
    #[error("runtime error")]
    Runtime,
    #[error("runtime error: {0}")]
    RuntimeMessage(String),
    #[error("parse error")]
    Parse,
    #[error("parse error: {0}")]
    ParseMessage(String),
    #[error("reader decode error")]
    ReaderDecode,
}

impl Error {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgumentMessage(message.into())
    }

    pub fn unsupported_operation(message: impl Into<String>) -> Self {
        Self::UnsupportedOperationMessage(message.into())
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::InvalidStateMessage(message.into())
    }

    pub fn arithmetic(message: impl Into<String>) -> Self {
        Self::ArithmeticMessage(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFoundMessage(message.into())
    }

    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormatMessage(message.into())
    }

    pub fn checksum(message: impl Into<String>) -> Self {
        Self::ChecksumMessage(message.into())
    }

    pub fn reader(message: impl Into<String>) -> Self {
        Self::ReaderMessage(message.into())
    }

    pub fn writer(message: impl Into<String>) -> Self {
        Self::WriterMessage(message.into())
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::RuntimeMessage(message.into())
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::ParseMessage(message.into())
    }

    pub fn is_invalid_format(&self) -> bool {
        matches!(self, Self::InvalidFormat | Self::InvalidFormatMessage(_))
    }

    pub fn is_checksum(&self) -> bool {
        matches!(self, Self::Checksum | Self::ChecksumMessage(_))
    }
}
