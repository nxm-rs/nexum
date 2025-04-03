use nexum_apdu_core::StatusWord;
use thiserror::Error;

/// Result type for GlobalPlatform operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for GlobalPlatform operations
#[derive(Debug, Error)]
pub enum Error {
    /// Transport-related errors
    #[error(transparent)]
    Transport(#[from] nexum_apdu_core::transport::TransportError),

    /// Command-related errors
    #[error(transparent)]
    Command(#[from] nexum_apdu_core::command::error::CommandError),

    /// Response-related errors
    #[error(transparent)]
    Response(#[from] nexum_apdu_core::response::error::ResponseError),

    /// Status errors (for status words)
    #[error(transparent)]
    Status(#[from] nexum_apdu_core::response::error::StatusError),

    /// Processor-related errors
    #[error(transparent)]
    Processor(#[from] nexum_apdu_core::processor::error::ProcessorError),

    /// Secure protocol related errors
    #[error(transparent)]
    SecureProtocol(#[from] nexum_apdu_core::processor::error::SecureProtocolError),

    /// Secure channel not established
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

    /// Other error
    #[error("{0}")]
    Other(&'static str),

    /// String error message (only available with std)
    #[error("{0}")]
    Msg(String),

    /// Core error wrapper (needed for generic error conversion)
    #[error(transparent)]
    CoreError(#[from] nexum_apdu_core::Error),
}

impl Error {
    /// Create a new error with a string message
    pub fn msg<S: Into<String>>(msg: S) -> Self {
        Self::Msg(msg.into())
    }

    /// Add context to an error
    pub fn with_context<S: Into<String>>(self, context: S) -> Self {
        let context_str = context.into();
        match self {
            Self::Msg(msg) => Self::Msg(format!("{}: {}", context_str, msg)),
            Self::Other(msg) => Self::Msg(format!("{}: {}", context_str, msg)),
            other => {
                // For other error types, wrap them in a Msg error to add context
                Self::Msg(format!("{}: {}", context_str, other))
            }
        }
    }

    /// Check if this error represents a specific card status
    pub const fn is_status(&self, status: u16) -> bool {
        matches!(self, Self::CardStatus(sw) if sw.to_u16() == status)
    }

    /// Try to extract a status word if this error contains one
    pub const fn status_word(&self) -> Option<StatusWord> {
        match self {
            Self::CardStatus(sw) => Some(*sw),
            Self::Status(e) => Some(e.status_word()),
            _ => None,
        }
    }
}

// Implement conversions between error types
impl From<Error> for nexum_apdu_core::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Transport(e) => e.into(),
            Error::Command(e) => e.into(),
            Error::Response(e) => e.into(),
            Error::Status(e) => e.into(),
            Error::Processor(e) => e.into(),
            Error::SecureProtocol(e) => e.into(),
            Error::CoreError(e) => e,
            Error::NoSecureChannel => Self::other("Secure channel not established"),
            Error::Crypto(msg) => Self::other(format!("Cryptographic error: {}", msg)),
            Error::InvalidFormat(msg) => Self::other(format!("Invalid format: {}", msg)),
            Error::InvalidLength { expected, actual } => Self::other(format!(
                "Invalid length: expected {}, got {}",
                expected, actual
            )),
            Error::AuthenticationFailed(msg) => {
                Self::other(format!("Authentication failed: {}", msg))
            }
            Error::InvalidChallenge(msg) => Self::other(format!("Invalid challenge: {}", msg)),
            Error::InvalidResponse(msg) => Self::other(format!("Invalid response: {}", msg)),
            Error::UnsupportedScpVersion(ver) => {
                Self::other(format!("Unsupported SCP version: {}", ver))
            }
            Error::CapFile(msg) => Self::other(format!("CAP file error: {}", msg)),
            Error::Io(e) => Self::other(format!("I/O error: {}", e)),
            Error::CardStatus(sw) => Self::status(sw.sw1, sw.sw2),
            Error::Other(msg) => Self::other(msg),
            Error::Msg(msg) => Self::other(msg),
        }
    }
}

impl From<Error> for nexum_apdu_core::processor::ProcessorError {
    fn from(err: Error) -> Self {
        match err {
            Error::Transport(e) => Self::Transport(e),
            Error::Command(e) => Self::other(format!("Command error: {:?}", e)),
            Error::Response(e) => Self::InvalidResponse(e),
            Error::Status(e) => Self::other(format!("Status error: {:?}", e)),
            Error::Processor(e) => e,
            Error::SecureProtocol(e) => Self::from(e),
            Error::CoreError(e) => match e {
                nexum_apdu_core::Error::Processor(pe) => pe,
                _ => Self::other(format!("Core error: {:?}", e)),
            },
            Error::NoSecureChannel => Self::session("Secure channel not established"),
            Error::Crypto(msg) => Self::other(format!("Crypto error: {}", msg)),
            Error::InvalidFormat(msg) => Self::other(format!("Invalid format: {}", msg)),
            Error::InvalidLength { expected, actual } => Self::other(format!(
                "Invalid length: expected {}, got {}",
                expected, actual
            )),
            Error::AuthenticationFailed(msg) => Self::authentication_failed(msg),
            Error::InvalidChallenge(msg) => Self::other(format!("Invalid challenge: {}", msg)),
            Error::InvalidResponse(msg) => Self::other(format!("Invalid response: {}", msg)),
            Error::UnsupportedScpVersion(ver) => {
                Self::other(format!("Unsupported SCP version: {}", ver))
            }
            Error::CapFile(msg) => Self::other(format!("CAP file error: {}", msg)),
            Error::Io(e) => Self::other(format!("I/O error: {}", e)),
            Error::CardStatus(sw) => Self::other(format!("Card status: {:?}", sw)),
            Error::Other(msg) => Self::other(msg),
            Error::Msg(msg) => Self::other(msg),
        }
    }
}
