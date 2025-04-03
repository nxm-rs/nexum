//! Error types specific to APDU commands

/// Error for APDU command processing
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// Invalid command length
    #[error("Invalid command length: {0}")]
    InvalidLength(usize),
}
