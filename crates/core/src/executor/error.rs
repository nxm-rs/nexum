//! Error types specific to APDU execution

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

use crate::command::error::CommandError;
use crate::processor::error::ProcessorError;
use crate::response::error::{ResponseError, StatusError};
use crate::transport::error::TransportError;

/// Error type for execution operations
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum ExecutionError {
    /// Transport error
    #[cfg_attr(feature = "std", error(transparent))]
    Transport(#[from] TransportError),

    /// Response parsing error
    #[cfg_attr(feature = "std", error(transparent))]
    Response(#[from] ResponseError),

    /// Command error
    #[cfg_attr(feature = "std", error(transparent))]
    Command(#[from] CommandError),

    /// Status error
    #[cfg_attr(feature = "std", error(transparent))]
    Status(#[from] StatusError),

    /// Processor error
    #[cfg_attr(feature = "std", error(transparent))]
    Processor(#[from] ProcessorError),

    /// Secure channel required
    #[cfg_attr(feature = "std", error("Secure channel required for command"))]
    SecureChannelRequired,

    /// Chain error occurred during a command chain
    #[cfg_attr(feature = "std", error("Command chain error: {0}"))]
    Chain(&'static str),

    /// Invalid response format
    #[cfg_attr(feature = "std", error("Invalid response format"))]
    InvalidResponseFormat,

    /// Command specific error
    #[cfg_attr(feature = "std", error("Command error: {0}"))]
    CommandSpecific(&'static str),

    /// Cancelled operation
    #[cfg_attr(feature = "std", error("Operation cancelled"))]
    Cancelled,

    /// Timeout
    #[cfg_attr(feature = "std", error("Operation timed out"))]
    Timeout,

    /// Other error with message (for std environments)
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("{0}"))]
    Other(String),

    /// Other error without message (for no-std environments)
    #[cfg(not(feature = "std"))]
    #[cfg_attr(feature = "std", error("Unknown execution error"))]
    Other,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "{}", e),
            Self::Response(e) => write!(f, "{}", e),
            Self::Command(e) => write!(f, "{}", e),
            Self::Status(e) => write!(f, "{}", e),
            Self::Processor(e) => write!(f, "{}", e),
            Self::SecureChannelRequired => write!(f, "Secure channel required for command"),
            Self::Chain(msg) => write!(f, "Command chain error: {}", msg),
            Self::InvalidResponseFormat => write!(f, "Invalid response format"),
            Self::CommandSpecific(msg) => write!(f, "Command error: {}", msg),
            Self::Cancelled => write!(f, "Operation cancelled"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::Other => write!(f, "Unknown execution error"),
        }
    }
}

impl ExecutionError {
    /// Create a new command specific error
    pub const fn command_error(message: &'static str) -> Self {
        Self::CommandSpecific(message)
    }

    /// Create a new chain error
    pub const fn chain_error(message: &'static str) -> Self {
        Self::Chain(message)
    }

    /// Check if this error is due to a status word
    pub const fn has_status(&self) -> bool {
        matches!(self, Self::Status(_))
    }

    /// Get the status word if this is a status error
    pub const fn status_word(&self) -> Option<crate::response::status::StatusWord> {
        match self {
            Self::Status(e) => Some(e.status_word()),
            _ => None,
        }
    }

    /// Check if this error has the given status word
    pub fn has_status_code(&self, sw: u16) -> bool {
        self.status_word().is_some_and(|status| status.to_u16() == sw)
    }

    /// Create a general other error
    #[cfg(feature = "std")]
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}
