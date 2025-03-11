//! Error types for command processors

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

use crate::transport::error::TransportError;

/// Error type for command processors
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum ProcessorError {
    /// Underlying transport error
    #[cfg_attr(feature = "std", error(transparent))]
    Transport(#[from] TransportError),

    /// Authentication failed
    #[cfg_attr(feature = "std", error("Authentication failed: {0}"))]
    AuthenticationFailed(&'static str),

    /// Session error
    #[cfg_attr(feature = "std", error("Session error: {0}"))]
    Session(&'static str),

    /// Secure messaging error
    #[cfg_attr(feature = "std", error("Secure messaging error: {0}"))]
    SecureMessaging(&'static str),

    /// Protocol error
    #[cfg_attr(feature = "std", error("Protocol error: {0}"))]
    Protocol(&'static str),

    /// Invalid response
    #[cfg_attr(feature = "std", error("Invalid response: {0}"))]
    InvalidResponse(&'static str),

    /// Chain limit exceeded
    #[cfg_attr(feature = "std", error("Chain limit exceeded"))]
    ChainLimitExceeded,

    /// Other error with message (for std environments)
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("{0}"))]
    Other(String),

    /// Other error without message (for no-std environments)
    #[cfg(not(feature = "std"))]
    #[cfg_attr(feature = "std", error("Unknown processor error"))]
    Other,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for ProcessorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "{}", e),
            Self::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            Self::Session(msg) => write!(f, "Session error: {}", msg),
            Self::SecureMessaging(msg) => write!(f, "Secure messaging error: {}", msg),
            Self::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            Self::ChainLimitExceeded => write!(f, "Chain limit exceeded"),
            Self::Other => write!(f, "Unknown processor error"),
        }
    }
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

    /// Create a new secure messaging error
    pub const fn secure_messaging(message: &'static str) -> Self {
        Self::SecureMessaging(message)
    }

    /// Create a new protocol error
    pub const fn protocol(message: &'static str) -> Self {
        Self::Protocol(message)
    }

    /// Create a new invalid response error
    pub const fn invalid_response(message: &'static str) -> Self {
        Self::InvalidResponse(message)
    }

    /// Create a general other error
    #[cfg(feature = "std")]
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}
