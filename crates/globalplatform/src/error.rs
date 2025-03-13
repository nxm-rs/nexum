//! Error types for GlobalPlatform operations

use nexum_apdu_core::{Error as ApduError, StatusWord};

#[cfg(feature = "std")]
use thiserror::Error;

/// Result type for GlobalPlatform operations
pub type Result<T> = core::result::Result<T, Error>;

/// Error type for GlobalPlatform operations
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum Error {
    /// APDU command or transport error
    #[cfg_attr(feature = "std", error("APDU error: {0}"))]
    Apdu(ApduError),

    /// Secure channel not established
    #[cfg_attr(feature = "std", error("Secure channel not established"))]
    NoSecureChannel,

    /// Cryptographic operation failed
    #[cfg_attr(feature = "std", error("Cryptographic error: {0}"))]
    Crypto(&'static str),

    /// Invalid or unsupported data format
    #[cfg_attr(feature = "std", error("Invalid data format: {0}"))]
    InvalidFormat(&'static str),

    /// Wrong data length
    #[cfg_attr(
        feature = "std",
        error("Invalid length: expected {expected}, got {actual}")
    )]
    InvalidLength {
        /// Expected length
        expected: usize,
        /// Actual length
        actual: usize,
    },

    /// Card authentication failed
    #[cfg_attr(feature = "std", error("Card authentication failed: {0}"))]
    AuthenticationFailed(&'static str),

    /// Invalid challenge
    #[cfg_attr(feature = "std", error("Invalid challenge: {0}"))]
    InvalidChallenge(&'static str),

    /// Invalid response
    #[cfg_attr(feature = "std", error("Invalid response: {0}"))]
    InvalidResponse(&'static str),

    /// Unsupported SCP version
    #[cfg_attr(feature = "std", error("Unsupported SCP version: {0}"))]
    UnsupportedScpVersion(u8),

    /// CAP file error
    #[cfg_attr(feature = "std", error("CAP file error: {0}"))]
    CapFile(&'static str),

    /// I/O error with CAP file
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("I/O error: {0}"))]
    Io(#[from] std::io::Error),

    /// Response indicates an error condition
    #[cfg_attr(feature = "std", error("Card returned error status: {0}"))]
    CardStatus(StatusWord),

    /// Other error
    #[cfg_attr(feature = "std", error("{0}"))]
    Other(&'static str),

    /// String error message (only available with std)
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("{0}"))]
    Msg(String),
}

impl From<ApduError> for Error {
    fn from(err: ApduError) -> Self {
        Self::Apdu(err)
    }
}

#[cfg(feature = "std")]
impl Error {
    /// Create a new error with a string message
    pub fn msg<S: Into<String>>(msg: S) -> Self {
        Self::Msg(msg.into())
    }
}
