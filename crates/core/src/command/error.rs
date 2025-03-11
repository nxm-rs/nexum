//! Error types specific to APDU commands

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

/// Error for APDU command processing
#[derive(Debug)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum CommandError {
    /// Invalid command format
    #[cfg_attr(feature = "std", error("Invalid command format"))]
    InvalidFormat,

    /// Invalid command length
    #[cfg_attr(feature = "std", error("Invalid command length: {0}"))]
    InvalidLength(usize),

    /// Data too long
    #[cfg_attr(feature = "std", error("Data too long: {0} bytes (max {1})"))]
    DataTooLong(usize, usize),

    /// Missing expected data
    #[cfg_attr(feature = "std", error("Missing expected data"))]
    MissingData,

    /// Invalid CLA byte
    #[cfg_attr(feature = "std", error("Invalid CLA byte: {0:#04X}"))]
    InvalidCla(u8),

    /// Invalid INS byte
    #[cfg_attr(feature = "std", error("Invalid INS byte: {0:#04X}"))]
    InvalidIns(u8),

    /// Parse error
    #[cfg_attr(feature = "std", error("Parse error: {0}"))]
    Parse(&'static str),

    /// Security level too low for command
    #[cfg_attr(feature = "std", error("Security level too low for command"))]
    SecurityLevel,

    /// Command not supported
    #[cfg_attr(feature = "std", error("Command not supported"))]
    NotSupported,

    /// Other error with message (for std environments)
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", error("{0}"))]
    Other(String),

    /// Other error without message (for no-std environments)
    #[cfg(not(feature = "std"))]
    #[cfg_attr(feature = "std", error("Unknown command error"))]
    Other,
}

#[cfg(not(feature = "std"))]
impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid command format"),
            Self::InvalidLength(len) => write!(f, "Invalid command length: {}", len),
            Self::DataTooLong(actual, max) => {
                write!(f, "Data too long: {} bytes (max {})", actual, max)
            }
            Self::MissingData => write!(f, "Missing expected data"),
            Self::InvalidCla(cla) => write!(f, "Invalid CLA byte: {:#04X}", cla),
            Self::InvalidIns(ins) => write!(f, "Invalid INS byte: {:#04X}", ins),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::SecurityLevel => write!(f, "Security level too low for command"),
            Self::NotSupported => write!(f, "Command not supported"),
            Self::Other => write!(f, "Unknown command error"),
        }
    }
}

impl CommandError {
    /// Create a parse error with a message
    pub const fn parse(message: &'static str) -> Self {
        Self::Parse(message)
    }

    /// Create a data too long error
    pub const fn data_too_long(actual: usize, max: usize) -> Self {
        Self::DataTooLong(actual, max)
    }

    /// Create a general other error
    #[cfg(feature = "std")]
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other(message.into())
    }
}
