//! Error types specific to APDU responses

use super::status::StatusWord;

/// Error for status words in APDU responses
#[derive(Debug, Clone, thiserror::Error)]
pub struct StatusError {
    /// Status word that caused the error
    pub status: StatusWord,
    /// Optional error message
    pub message: Option<&'static str>,
}

impl std::fmt::Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status error {}", self.status)?;
        if let Some(msg) = self.message {
            write!(f, ": {}", msg)?;
        }
        Ok(())
    }
}

impl StatusError {
    /// Create a new status error
    pub const fn new(sw1: u8, sw2: u8) -> Self {
        Self {
            status: StatusWord::new(sw1, sw2),
            message: None,
        }
    }

    /// Create a new status error with a message
    pub const fn with_message(sw1: u8, sw2: u8, message: &'static str) -> Self {
        Self {
            status: StatusWord::new(sw1, sw2),
            message: Some(message),
        }
    }

    /// Get the status word
    pub const fn status_word(&self) -> StatusWord {
        self.status
    }
}

/// Error for APDU response processing
#[derive(Debug, thiserror::Error)]
pub enum ResponseError {
    /// Incomplete response (less than 2 bytes)
    #[error("Incomplete response")]
    Incomplete,

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(&'static str),

    /// Status error
    #[error(transparent)]
    Status(#[from] StatusError),

    /// Buffer too small
    #[error("Buffer too small")]
    BufferTooSmall,

    /// Status word error with custom message
    #[error("Response error: {0}")]
    Message(String),

    /// Other error
    #[error("Unknown response error")]
    Other,
}

impl ResponseError {
    /// Create a new status error
    pub const fn status(sw1: u8, sw2: u8) -> Self {
        Self::Status(StatusError::new(sw1, sw2))
    }

    /// Create a new status error with a message
    pub const fn status_with_message(sw1: u8, sw2: u8, message: &'static str) -> Self {
        Self::Status(StatusError::with_message(sw1, sw2, message))
    }

    /// Create a parse error with a message
    pub const fn parse(message: &'static str) -> Self {
        Self::Parse(message)
    }

    /// Create an invalid response error with a message
    pub const fn invalid_response(message: &'static str) -> Self {
        Self::Parse(message)
    }

    /// Check if this error has the given status word
    pub const fn has_status(&self, sw: u16) -> bool {
        if let Self::Status(status_error) = self {
            status_error.status_word().to_u16() == sw
        } else {
            false
        }
    }

    /// Create a message error
    pub fn message<S: Into<String>>(message: S) -> Self {
        Self::Message(message.into())
    }
}
