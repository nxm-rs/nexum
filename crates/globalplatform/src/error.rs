//! Error types for GlobalPlatform operations
//!
//! This module provides error types specific to GlobalPlatform card
//! management operations.

use nexum_apdu_core::prelude::*;
use thiserror::Error;

use crate::commands::{
    delete::DeleteError, external_authenticate::ExternalAuthenticateError,
    get_response::GetResponseError, get_status::GetStatusError,
    initialize_update::InitializeUpdateError, install::InstallError, load::LoadError,
    put_key::PutKeyError, select::SelectError, store_data::StoreDataError,
};

/// Result type for GlobalPlatform operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for GlobalPlatform operations
///
/// This enum represents all possible errors that can occur during GlobalPlatform
/// card management operations, including communication errors, cryptographic errors,
/// and specific command errors returned by the card.
#[derive(Debug, Error)]
pub enum Error {
    /// Core error from nexum_apdu_core
    #[error(transparent)]
    Core(#[from] nexum_apdu_core::Error),

    /// Secure Channel not established
    #[error("Secure channel not established")]
    NoSecureChannel,

    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(&'static str),

    /// Invalid or unsupported data format
    #[error("Invalid data format: {0}")]
    InvalidFormat(&'static str),

    /// Wrong data length
    #[error("Invalid length: expected {expected}, got {actual}")]
    InvalidLength {
        /// Expected length
        expected: usize,
        /// Actual length
        actual: usize,
    },

    /// Card authentication failed
    #[error("Card authentication failed: {0}")]
    AuthenticationFailed(&'static str),

    /// Invalid challenge
    #[error("Invalid challenge: {0}")]
    InvalidChallenge(&'static str),

    /// Invalid response
    #[error("Invalid response: {0}")]
    InvalidResponse(&'static str),

    /// Unsupported SCP version
    #[error("Unsupported SCP version: {0}")]
    UnsupportedScpVersion(u8),

    /// CAP file error
    #[error("CAP file error: {0}")]
    CapFile(&'static str),

    /// I/O error with CAP file
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Response indicates an error condition
    #[error("Card returned error status: {0}")]
    CardStatus(StatusWord),

    /// Session creation failed
    #[error("Failed to create secure channel session: {0}")]
    SessionCreationFailed(&'static str),

    /// Context with source error
    #[error("{context}: {source}")]
    Context {
        /// Contextual message
        context: String,
        /// Source error
        source: Box<Self>,
    },

    /// Other error with dynamic message
    #[error("{0}")]
    Message(String),

    /// Other error with static message
    #[error("{0}")]
    Other(&'static str),

    // Command-specific errors
    
    /// Error from DELETE command
    #[error(transparent)]
    DeleteError(#[from] DeleteError),

    /// Error from EXTERNAL AUTHENTICATE command
    #[error(transparent)]
    ExternalAuthenticateError(#[from] ExternalAuthenticateError),

    /// Error from GET RESPONSE command
    #[error(transparent)]
    GetResponseError(#[from] GetResponseError),

    /// Error from GET STATUS command
    #[error(transparent)]
    GetStatusError(#[from] GetStatusError),

    /// Error from INITIALIZE UPDATE command
    #[error(transparent)]
    InitializeUpdateError(#[from] InitializeUpdateError),

    /// Error from INSTALL command
    #[error(transparent)]
    InstallError(#[from] InstallError),

    /// Error from LOAD command
    #[error(transparent)]
    LoadError(#[from] LoadError),

    /// Error from PUT KEY command
    #[error(transparent)]
    PutKeyError(#[from] PutKeyError),

    /// Error from SELECT command
    #[error(transparent)]
    SelectError(#[from] SelectError),

    /// Error from STORE DATA command
    #[error(transparent)]
    StoreDataError(#[from] StoreDataError),
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
}

/// Extension trait for Result with context addition
pub trait ResultExt<T> {
    /// Add context to an error
    fn context<S: Into<String>>(self, context: S) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context<S: Into<String>>(self, context: S) -> Self {
        self.map_err(|e| e.with_context(context))
    }
}

/// Extension trait for nexum_apdu_core::Result
pub trait CoreResultExt<T> {
    /// Convert core result to GlobalPlatform result
    fn to_gp(self) -> Result<T>;
}

impl<T> CoreResultExt<T> for std::result::Result<T, nexum_apdu_core::Error> {
    fn to_gp(self) -> Result<T> {
        self.map_err(Error::from)
    }
}
