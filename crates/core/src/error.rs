//! Unified error type for APDU operations

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "std")] {
        use thiserror::Error;
        use std::string::String;
    } else {
        use alloc::string::String;
        use core::fmt;
    }
}

use crate::response::status::StatusWord;

/// The main error type for APDU operations
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum Error {
    /// Transport-related errors
    #[cfg_attr(feature = "std", error(transparent))]
    Transport(#[from] crate::transport::error::TransportError),

    /// Command-related errors
    #[cfg_attr(feature = "std", error(transparent))]
    Command(#[from] crate::command::error::CommandError),

    /// Response-related errors
    #[cfg_attr(feature = "std", error(transparent))]
    Response(#[from] crate::response::error::ResponseError),

    /// Execution-related errors
    #[cfg_attr(feature = "std", error(transparent))]
    Execution(#[from] crate::executor::error::ExecutionError),

    /// Status errors (for status words)
    #[cfg_attr(feature = "std", error(transparent))]
    Status(#[from] crate::response::error::StatusError),

    /// Processor-related errors
    #[cfg_attr(feature = "std", error(transparent))]
    Processor(#[from] crate::processor::error::ProcessorError),

    /// Parse errors
    #[cfg_attr(feature = "std", error("Parse error: {0}"))]
    Parse(&'static str),

    /// Other errors with message (for std environments)
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("{0}"))]
    Other(String),

    /// Other errors without message (for no-std environments)
    #[cfg(not(feature = "std"))]
    #[cfg_attr(feature = "std", error("Unknown error"))]
    Other,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Transport(e) => write!(f, "{}", e),
            Error::Command(e) => write!(f, "{}", e),
            Error::Response(e) => write!(f, "{}", e),
            Error::Execution(e) => write!(f, "{}", e),
            Error::Status(e) => write!(f, "{}", e),
            Error::Processor(e) => write!(f, "{}", e),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::Other => write!(f, "Unknown error"),
        }
    }
}

impl Error {
    /// Create a new error with the given status word
    pub const fn status(sw1: u8, sw2: u8) -> Self {
        Self::Status(crate::response::error::StatusError::new(sw1, sw2))
    }

    /// Create a new error with the given status word and message
    pub const fn status_with_message(sw1: u8, sw2: u8, message: &'static str) -> Self {
        Self::Status(crate::response::error::StatusError::with_message(
            sw1, sw2, message,
        ))
    }

    /// Check if this error has the given status word
    pub const fn has_status(&self, sw: u16) -> bool {
        if let Self::Status(status_error) = self {
            status_error.status_word().to_u16() == sw
        } else {
            false
        }
    }

    /// Get the status word if this is a status error
    pub const fn status_word(&self) -> Option<StatusWord> {
        if let Self::Status(status_error) = self {
            Some(status_error.status_word())
        } else {
            None
        }
    }

    /// Create a generic other error
    #[cfg(feature = "std")]
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}

/// Result type for APDU operations
pub type Result<T> = core::result::Result<T, Error>;
