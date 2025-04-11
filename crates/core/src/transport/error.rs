//! Error types specific to card transport

/// Transport error type
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
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

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Other error with message
    #[error("{0}")]
    Other(String),
}
