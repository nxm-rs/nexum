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
    InvalidResponse(#[from] ResponseError),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(&'static str),

    /// Session error
    #[error("Session error: {0}")]
    Session(&'static str),

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
    /// Create a new authentication failed error
    pub const fn authentication_failed(message: &'static str) -> Self {
        Self::AuthenticationFailed(message)
    }

    /// Create a new session error
    pub const fn session(message: &'static str) -> Self {
        Self::Session(message)
    }

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

    /// Processor error
    #[error("Processor error: {0}")]
    Processor(#[from] ProcessorError),

    /// Other error with message (for std environments)
    #[error("{0}")]
    Other(String),
}

impl SecureProtocolError {
    /// Create a new protocol error
    pub const fn protocol(message: &'static str) -> Self {
        Self::Protocol(message)
    }

    /// Create a general other error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}

impl From<SecureProtocolError> for ProcessorError {
    fn from(error: SecureProtocolError) -> Self {
        match error {
            SecureProtocolError::Protocol(message) => Self::Protocol(message),
            SecureProtocolError::Processor(error) => Self::Other(error.to_string()),
            SecureProtocolError::Other(message) => Self::Other(message),
        }
    }
}
