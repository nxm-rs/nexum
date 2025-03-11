//! Error types for PC/SC transport

use apdu_core::transport::error::TransportError;

#[cfg(feature = "std")]
use core::fmt;

#[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use core::fmt;

/// PC/SC-specific errors
#[derive(Debug)]
pub enum PcscError {
    /// PC/SC error
    #[cfg(feature = "std")]
    Pcsc(pcsc::Error),

    /// No readers available
    NoReadersAvailable,

    /// Reader not found
    #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
    ReaderNotFound(String),

    /// Reader not found (no alloc version)
    #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
    ReaderNotFound,

    /// No card present in reader
    #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
    NoCard(String),

    /// No card present in reader (no alloc version)
    #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
    NoCard,

    /// Card was reset
    CardReset,

    /// Card was removed
    CardRemoved,

    /// Transaction already in progress
    TransactionInProgress,

    /// No active transaction
    NoTransaction,

    /// Other error
    #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
    Other(String),

    /// Other error (no-alloc version)
    #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
    Other,
}

impl fmt::Display for PcscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "std")]
            Self::Pcsc(e) => write!(f, "PC/SC error: {}", e),
            Self::NoReadersAvailable => write!(f, "No readers available"),
            #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
            Self::ReaderNotFound(r) => write!(f, "Reader not found: {}", r),
            #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
            Self::ReaderNotFound => write!(f, "Reader not found"),
            #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
            Self::NoCard(r) => write!(f, "No card present in reader: {}", r),
            #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
            Self::NoCard => write!(f, "No card present in reader"),
            Self::CardReset => write!(f, "Card was reset"),
            Self::CardRemoved => write!(f, "Card was removed"),
            Self::TransactionInProgress => write!(f, "Transaction already in progress"),
            Self::NoTransaction => write!(f, "No active transaction"),
            #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
            Self::Other(msg) => write!(f, "{}", msg),
            #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
            Self::Other => write!(f, "Unknown PC/SC error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PcscError {}

#[cfg(feature = "std")]
impl From<pcsc::Error> for PcscError {
    fn from(error: pcsc::Error) -> Self {
        Self::Pcsc(error)
    }
}

impl From<PcscError> for TransportError {
    fn from(error: PcscError) -> Self {
        match error {
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::NoSmartcard) => TransportError::Device,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::ResetCard) => TransportError::Device,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::RemovedCard) => TransportError::Device,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::Timeout) => TransportError::Timeout,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::InsufficientBuffer) => TransportError::BufferTooSmall,
            #[cfg(feature = "std")]
            PcscError::Pcsc(e) => {
                #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
                {
                    TransportError::Other(alloc::format!("PC/SC error: {}", e))
                }
                #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
                {
                    TransportError::Other
                }
            }
            PcscError::NoReadersAvailable => TransportError::Connection,
            #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
            PcscError::ReaderNotFound(_) => TransportError::Connection,
            #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
            PcscError::ReaderNotFound => TransportError::Connection,
            #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
            PcscError::NoCard(_) => TransportError::Device,
            #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
            PcscError::NoCard => TransportError::Device,
            PcscError::CardReset | PcscError::CardRemoved => TransportError::Device,
            PcscError::TransactionInProgress | PcscError::NoTransaction => {
                TransportError::Transmission
            }
            #[cfg(any(feature = "std", feature = "alloc", feature = "wasm"))]
            PcscError::Other(msg) => TransportError::Other(msg),
            #[cfg(not(any(feature = "std", feature = "alloc", feature = "wasm")))]
            PcscError::Other => TransportError::Other,
        }
    }
}
