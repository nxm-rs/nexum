//! Error types for PC/SC transport

use std::fmt;

use nexum_apdu_core::transport::error::TransportError;

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

impl From<PcscError> for TransportError {
    fn from(error: PcscError) -> Self {
        match error {
            PcscError::Pcsc(pcsc::Error::NoSmartcard) => Self::Device,
            PcscError::Pcsc(pcsc::Error::ResetCard) => Self::Device,
            PcscError::Pcsc(pcsc::Error::RemovedCard) => Self::Device,
            PcscError::Pcsc(pcsc::Error::Timeout) => Self::Timeout,
            PcscError::Pcsc(pcsc::Error::InsufficientBuffer) => Self::BufferTooSmall,
            PcscError::Pcsc(e) => Self::Other(format!("PC/SC error: {}", e)),
            PcscError::NoReadersAvailable => Self::Connection,
            PcscError::ReaderNotFound(_) => Self::Connection,
            PcscError::NoCard(_) => Self::Device,
            PcscError::CardReset | PcscError::CardRemoved => Self::Device,
            PcscError::TransactionInProgress | PcscError::NoTransaction => Self::Transmission,
            PcscError::Other(msg) => Self::Other(msg),
        }
    }
}
