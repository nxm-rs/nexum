//! Error types specific to card transport

/// Transport error type
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Connection error
    #[error("Failed to connect to device")]
    Connection,

    /// Transmission error
    #[error("Failed to transmit data")]
    Transmission,

    /// Device error
    #[error("Device error")]
    Device,

    /// Buffer too small
    #[error("Buffer too small")]
    BufferTooSmall,

    /// Driver error (with code)
    #[error("Driver error code: {0}")]
    Driver(i32),

    /// Status word error
    #[error("Status word error: {0:#06X}")]
    StatusWord(u16),

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Cancelled operation
    #[error("Operation cancelled")]
    Cancelled,

    /// Other error with message
    #[error("{0}")]
    Other(String),
}

impl TransportError {
    /// Create a new status word error
    pub const fn status_word(sw: u16) -> Self {
        Self::StatusWord(sw)
    }

    /// Create a new status word error from individual bytes
    pub const fn status_word_bytes(sw1: u8, sw2: u8) -> Self {
        Self::StatusWord(((sw1 as u16) << 8) | (sw2 as u16))
    }

    /// Create a new driver error
    pub const fn driver(code: i32) -> Self {
        Self::Driver(code)
    }

    /// Check if this is a status word error
    pub const fn is_status_word(&self) -> bool {
        matches!(self, Self::StatusWord(_))
    }

    /// Get the status word if this is a status word error
    pub const fn get_status_word(&self) -> Option<u16> {
        match self {
            Self::StatusWord(sw) => Some(*sw),
            _ => None,
        }
    }

    /// Create a general other error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}
