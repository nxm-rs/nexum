//! Error types for PC/SC transport

use nexum_apdu_core::transport::error::TransportError;

#[cfg(feature = "std")]
use core::fmt;

#[cfg(any(feature = "std", feature = "alloc"))]
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
    #[cfg(any(feature = "std", feature = "alloc"))]
    ReaderNotFound(String),

    /// Reader not found (no alloc version)
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    ReaderNotFound,

    /// No card present in reader
    #[cfg(any(feature = "std", feature = "alloc"))]
    NoCard(String),

    /// No card present in reader (no alloc version)
    #[cfg(not(any(feature = "std", feature = "alloc")))]
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
    #[cfg(any(feature = "std", feature = "alloc"))]
    Other(String),

    /// Other error (no-alloc version)
    #[cfg(not(any(feature = "std", feature = "alloc")))]
    Other,
}

impl fmt::Display for PcscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "std")]
            Self::Pcsc(e) => write!(f, "PC/SC error: {}", e),
            Self::NoReadersAvailable => write!(f, "No readers available"),
            #[cfg(any(feature = "std", feature = "alloc"))]
            Self::ReaderNotFound(r) => write!(f, "Reader not found: {}", r),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            Self::ReaderNotFound => write!(f, "Reader not found"),
            #[cfg(any(feature = "std", feature = "alloc"))]
            Self::NoCard(r) => write!(f, "No card present in reader: {}", r),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            Self::NoCard => write!(f, "No card present in reader"),
            Self::CardReset => write!(f, "Card was reset"),
            Self::CardRemoved => write!(f, "Card was removed"),
            Self::TransactionInProgress => write!(f, "Transaction already in progress"),
            Self::NoTransaction => write!(f, "No active transaction"),
            #[cfg(any(feature = "std", feature = "alloc"))]
            Self::Other(msg) => write!(f, "{}", msg),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
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
            PcscError::Pcsc(pcsc::Error::NoSmartcard) => Self::Device,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::ResetCard) => Self::Device,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::RemovedCard) => Self::Device,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::Timeout) => Self::Timeout,
            #[cfg(feature = "std")]
            PcscError::Pcsc(pcsc::Error::InsufficientBuffer) => Self::BufferTooSmall,
            #[cfg(feature = "std")]
            PcscError::Pcsc(e) => {
                #[cfg(any(feature = "std", feature = "alloc"))]
                {
                    Self::Other(alloc::format!("PC/SC error: {}", e))
                }
                #[cfg(not(any(feature = "std", feature = "alloc")))]
                {
                    TransportError::Other
                }
            }
            PcscError::NoReadersAvailable => Self::Connection,
            #[cfg(any(feature = "std", feature = "alloc"))]
            PcscError::ReaderNotFound(_) => Self::Connection,
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            PcscError::ReaderNotFound => TransportError::Connection,
            #[cfg(any(feature = "std", feature = "alloc"))]
            PcscError::NoCard(_) => Self::Device,
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            PcscError::NoCard => TransportError::Device,
            PcscError::CardReset | PcscError::CardRemoved => Self::Device,
            PcscError::TransactionInProgress | PcscError::NoTransaction => Self::Transmission,
            #[cfg(any(feature = "std", feature = "alloc"))]
            PcscError::Other(msg) => Self::Other(msg),
            #[cfg(not(any(feature = "std", feature = "alloc")))]
            PcscError::Other => TransportError::Other,
        }
    }
}
