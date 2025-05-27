//! Error types for PC/SC transport

use std::fmt;

/// PC/SC-specific errors
#[derive(Debug, thiserror::Error)]
pub enum PcscError {
    /// PC/SC error
    Pcsc(#[from] pcsc::Error),

    /// No readers available
    NoReadersAvailable,

    /// Reader not found
    ReaderNotFound(String),

    /// No card present in reader
    NoCard(String),

    /// Card was reset
    CardReset,

    /// Card was removed
    CardRemoved,

    /// Transaction already in progress
    TransactionInProgress,

    /// No active transaction
    NoTransaction,

    /// Other error
    Other(String),
}

impl fmt::Display for PcscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pcsc(e) => write!(f, "PC/SC error: {}", e),
            Self::NoReadersAvailable => write!(f, "No readers available"),
            Self::ReaderNotFound(r) => write!(f, "Reader not found: {}", r),
            Self::NoCard(r) => write!(f, "No card present in reader: {}", r),
            Self::CardReset => write!(f, "Card was reset"),
            Self::CardRemoved => write!(f, "Card was removed"),
            Self::TransactionInProgress => write!(f, "Transaction already in progress"),
            Self::NoTransaction => write!(f, "No active transaction"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}
