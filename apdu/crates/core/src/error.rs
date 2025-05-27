//! Core error type for all APDU operations
//!
//! This module provides a centralized error type used throughout the nexum_apdu_core crate.
//! All error variants are consolidated here to simplify error handling and facilitate
//! better error bubbling up through the call stack.

use crate::response::status::StatusWord;
use crate::secure_channel::SecurityLevel;

/// Core error type that encompasses all possible errors in the crate
#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    //
    // Transport related errors
    //
    /// Failed to connect to the device
    #[error("Connection error: failed to connect to device")]
    ConnectionError,

    /// Failed to transmit data
    #[error("Transmission error: failed to transmit data")]
    TransmissionError,

    /// Device error
    #[error("Device error")]
    DeviceError,

    /// Buffer too small
    #[error("Buffer too small")]
    BufferTooSmall,

    /// Operation timed out
    #[error("Operation timed out")]
    Timeout,

    //
    // Response related errors
    //
    /// Parse error when processing response
    #[error("Parse error: {0}")]
    ParseError(&'static str),

    /// Status error from response
    #[error("Status error {status}, message: {message:?}")]
    StatusError {
        /// Status word that caused the error
        status: StatusWord,
        /// Optional error message
        message: Option<&'static str>,
    },

    //
    // Command related errors
    //
    /// Invalid command length
    #[error("Invalid command length: {0}")]
    InvalidCommandLength(usize),

    /// Invalid command data
    #[error("Invalid command data: {0}")]
    InvalidCommandData(&'static str),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    //
    // Processor related errors
    //
    /// Protocol error
    #[error("Protocol error: {0}")]
    ProtocolError(&'static str),

    /// Chain limit exceeded
    #[error("Chain limit exceeded")]
    ChainLimitExceeded,

    //
    // Secure channel related errors
    //
    /// Secure channel not established
    #[error("Secure channel not established")]
    SecureChannelNotEstablished,

    /// Insufficient security level
    #[error("Insufficient security level: required {required:?}, current {current:?}")]
    InsufficientSecurityLevel {
        /// Required security level
        required: SecurityLevel,
        /// Current security level
        current: SecurityLevel,
    },

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(&'static str),

    /// Session error
    #[error("Session error: {0}")]
    SessionError(&'static str),

    //
    // General errors
    //
    /// Context error with message and source error
    #[error("{context}: {source}")]
    Context {
        /// Contextual message
        context: String,
        /// Source error
        source: Box<Self>,
    },

    /// Other error with static message
    #[error("{0}")]
    Other(&'static str),

    /// Generic dynamic error with string message
    #[error("{0}")]
    Message(String),
}

impl Error {
    /// Create a new error with context information
    pub fn with_context<S: Into<String>>(self, context: S) -> Self {
        Self::Context {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Create a new error with a static message
    pub const fn other(message: &'static str) -> Self {
        Self::Other(message)
    }

    /// Create a new error with a dynamic message
    pub fn message<S: Into<String>>(message: S) -> Self {
        Self::Message(message.into())
    }

    /// Create a new status error
    pub const fn status(sw1: u8, sw2: u8) -> Self {
        Self::StatusError {
            status: StatusWord::new(sw1, sw2),
            message: None,
        }
    }

    /// Create a new status error with a message
    pub const fn status_with_message(sw1: u8, sw2: u8, message: &'static str) -> Self {
        Self::StatusError {
            status: StatusWord::new(sw1, sw2),
            message: Some(message),
        }
    }

    /// Create a new protocol error
    pub const fn protocol(message: &'static str) -> Self {
        Self::ProtocolError(message)
    }

    /// Create a new parse error
    pub const fn parse(message: &'static str) -> Self {
        Self::ParseError(message)
    }
}

/// Extension trait for Result with APDU Errors
pub trait ResultExt<T> {
    /// Add context to an error
    fn context<S: Into<String>>(self, context: S) -> Result<T, Error>;
}

impl<T> ResultExt<T> for Result<T, Error> {
    fn context<S: Into<String>>(self, context: S) -> Self {
        self.map_err(|e| e.with_context(context))
    }
}