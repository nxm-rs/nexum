//! Error types specific to APDU responses

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

use super::status::StatusWord;

/// Error for status words in APDU responses
#[derive(Debug, Clone)]
#[cfg_attr(feature = "std", derive(Error))]
pub struct StatusError {
    /// Status word that caused the error
    pub status: StatusWord,
    /// Optional error message
    pub message: Option<&'static str>,
}

#[cfg(feature = "std")]
impl std::fmt::Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status error {}", self.status)?;
        if let Some(msg) = self.message {
            write!(f, ": {}", msg)?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum ResponseError {
    /// Incomplete response (less than 2 bytes)
    #[cfg_attr(feature = "std", error("Incomplete response"))]
    Incomplete,

    /// Parse error
    #[cfg_attr(feature = "std", error("Parse error: {0}"))]
    Parse(&'static str),

    /// Status error
    #[cfg_attr(feature = "std", error(transparent))]
    Status(#[from] StatusError),

    /// Buffer too small
    #[cfg_attr(feature = "std", error("Buffer too small"))]
    BufferTooSmall,

    /// Status word error with custom message
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("Response error: {0}"))]
    Message(String),

    /// Other error
    #[cfg_attr(feature = "std", error("Unknown response error"))]
    Other,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete => write!(f, "Incomplete response"),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Status(e) => write!(f, "{}", e),
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::Other => write!(f, "Unknown response error"),
        }
    }
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

    /// Check if this error has the given status word
    pub const fn has_status(&self, sw: u16) -> bool {
        if let Self::Status(status_error) = self {
            status_error.status_word().to_u16() == sw
        } else {
            false
        }
    }

    /// Create a message error
    #[cfg(feature = "std")]
    pub fn message<S: Into<String>>(message: S) -> Self {
        Self::Message(message.into())
    }
}
