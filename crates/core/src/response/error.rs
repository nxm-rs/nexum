//! Error types specific to APDU responses

use derive_more::Display;

use crate::transport::TransportError;

use super::status::StatusWord;

/// Error for status words in APDU responses
#[derive(Debug, Clone, thiserror::Error, Display)]
#[display("Status error {}, message: {:?}", status, message)]
pub struct StatusError {
    /// Status word that caused the error
    pub status: StatusWord,
    /// Optional error message
    pub message: Option<&'static str>,
}

impl StatusError {
    /// Create a new status error
    pub const fn new(sw1: u8, sw2: u8) -> Self {
        Self {
            status: StatusWord::new(sw1, sw2),
            message: None,
        }
    }
}

/// Error for APDU response processing
#[derive(Debug, thiserror::Error)]
pub enum ResponseError {
    /// Underlying transport has caused an error
    #[error(transparent)]
    Transport(#[from] TransportError),

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
}

impl ResponseError {
    /// Create a new status error
    pub const fn status(sw1: u8, sw2: u8) -> Self {
        Self::Status(StatusError::new(sw1, sw2))
    }
}
