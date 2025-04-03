//! Unified error type for APDU operations

use crate::response::status::StatusWord;

/// The main error type for APDU operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport-related errors
    #[error(transparent)]
    Transport(#[from] crate::transport::error::TransportError),

    /// Command-related errors
    #[error(transparent)]
    Command(#[from] crate::command::error::CommandError),

    /// Response-related errors
    #[error(transparent)]
    Response(#[from] crate::response::error::ResponseError),

    /// Status errors (for status words)
    #[error(transparent)]
    Status(#[from] crate::response::error::StatusError),

    /// Processor-related errors
    #[error(transparent)]
    Processor(#[from] crate::processor::error::ProcessorError),

    /// Secure protocol related errors
    #[error(transparent)]
    SecureProtocol(#[from] crate::processor::error::SecureProtocolError),

    /// Parse errors
    #[error("Parse error: {0}")]
    Parse(&'static str),

    /// Other errors with message (for std environments)
    #[error("{0}")]
    Other(String),
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
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}

/// Result type for APDU operations
pub type Result<T> = std::result::Result<T, Error>;
