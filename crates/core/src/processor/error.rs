//! Error types for command processors

use crate::{response::error::ResponseError, transport::error::TransportError};

/// Error type for command processors
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    /// Underlying transport error
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Invalid response
    #[error(transparent)]
    Response(#[from] ResponseError),

    /// Secure channel error
    #[error(transparent)]
    SecureChannel(#[from] SecureProtocolError),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(&'static str),

    /// Chain limit exceeded
    #[error("Chain limit exceeded")]
    ChainLimitExceeded,

    /// Other error with message (for std environments)
    #[error("{0}")]
    Other(String),
}

impl ProcessorError {
    /// Create a new protocol error
    pub const fn protocol(message: &'static str) -> Self {
        Self::Protocol(message)
    }

    /// Create a general other error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}

/// Error type for secure protocols
#[derive(Debug, thiserror::Error)]
pub enum SecureProtocolError {
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(&'static str),

    /// Response error
    #[error("Response error: {0}")]
    Response(#[from] ResponseError),

    /// Insufficient security level
    #[error("Current security level is insufficient")]
    InsufficientSecurityLevel,

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(&'static str),

    /// Session error
    #[error("Session error: {0}")]
    Session(&'static str),

    /// Other error with message (for std environments)
    #[error("{0}")]
    Other(String),
}
