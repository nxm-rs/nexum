//! Error types specific to card transport

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

/// Transport error type
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum TransportError {
    /// Connection error
    #[cfg_attr(feature = "std", error("Failed to connect to device"))]
    Connection,

    /// Transmission error
    #[cfg_attr(feature = "std", error("Failed to transmit data"))]
    Transmission,

    /// Device error
    #[cfg_attr(feature = "std", error("Device error"))]
    Device,

    /// Buffer too small
    #[cfg_attr(feature = "std", error("Buffer too small"))]
    BufferTooSmall,

    /// Driver error (with code)
    #[cfg_attr(feature = "std", error("Driver error code: {0}"))]
    Driver(i32),

    /// Status word error
    #[cfg_attr(feature = "std", error("Status word error: {0:#06X}"))]
    StatusWord(u16),

    /// Timeout error
    #[cfg_attr(feature = "std", error("Operation timed out"))]
    Timeout,

    /// Cancelled operation
    #[cfg_attr(feature = "std", error("Operation cancelled"))]
    Cancelled,

    /// Other error with message
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("{0}"))]
    Other(String),

    /// Other error without message (for no_std)
    #[cfg(not(feature = "std"))]
    #[cfg_attr(feature = "std", error("Unknown transport error"))]
    Other,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection => write!(f, "Failed to connect to device"),
            Self::Transmission => write!(f, "Failed to transmit data"),
            Self::Device => write!(f, "Device error"),
            Self::BufferTooSmall => write!(f, "Buffer too small"),
            Self::Driver(code) => write!(f, "Driver error code: {}", code),
            Self::StatusWord(sw) => write!(f, "Status word error: {:#06X}", sw),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::Cancelled => write!(f, "Operation cancelled"),
            Self::Other => write!(f, "Unknown transport error"),
        }
    }
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
    #[cfg(feature = "std")]
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}
