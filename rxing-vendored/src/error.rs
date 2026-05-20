use thiserror::Error as ThisError;

#[derive(ThisError, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    #[error("invalid argument{}", fmt_msg(.0))]
    InvalidArgument(Option<String>),
    #[error("unsupported operation{}", fmt_msg(.0))]
    UnsupportedOperation(Option<String>),
    #[error("invalid state{}", fmt_msg(.0))]
    InvalidState(Option<String>),
    #[error("arithmetic error{}", fmt_msg(.0))]
    Arithmetic(Option<String>),
    #[error("not found{}", fmt_msg(.0))]
    NotFound(Option<String>),
    #[error("invalid format{}", fmt_msg(.0))]
    InvalidFormat(Option<String>),
    #[error("checksum failed{}", fmt_msg(.0))]
    Checksum(Option<String>),
    #[error("reader error{}", fmt_msg(.0))]
    Reader(Option<String>),
    #[error("writer error{}", fmt_msg(.0))]
    Writer(Option<String>),
    #[error("runtime error{}", fmt_msg(.0))]
    Runtime(Option<String>),
    #[error("parse error{}", fmt_msg(.0))]
    Parse(Option<String>),
    #[error("reader decode error{}", fmt_msg(.0))]
    ReaderDecode(Option<String>),
}

fn fmt_msg(msg: &Option<String>) -> String {
    match msg {
        Some(m) => format!(": {m}"),
        None => String::new(),
    }
}

impl Error {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(Some(message.into()))
    }

    pub fn unsupported_operation(message: impl Into<String>) -> Self {
        Self::UnsupportedOperation(Some(message.into()))
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::InvalidState(Some(message.into()))
    }

    pub fn arithmetic(message: impl Into<String>) -> Self {
        Self::Arithmetic(Some(message.into()))
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(Some(message.into()))
    }

    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat(Some(message.into()))
    }

    pub fn checksum(message: impl Into<String>) -> Self {
        Self::Checksum(Some(message.into()))
    }

    pub fn reader(message: impl Into<String>) -> Self {
        Self::Reader(Some(message.into()))
    }

    pub fn writer(message: impl Into<String>) -> Self {
        Self::Writer(Some(message.into()))
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(Some(message.into()))
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(Some(message.into()))
    }

    pub fn is_invalid_format(&self) -> bool {
        matches!(self, Self::InvalidFormat(_))
    }

    pub fn is_checksum(&self) -> bool {
        matches!(self, Self::Checksum(_))
    }
}
