//! Configuration options for PC/SC transport

#[cfg(feature = "std")]
use pcsc::{Protocols as PcscProtocols, ShareMode as PcscShareMode};

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::string::String;
use alloc::vec::Vec;

/// Sharing mode for card connections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareMode {
    /// Exclusive access to the card
    Exclusive,
    /// Shared access to the card (default)
    Shared,
    /// Direct connection to the reader
    Direct,
}

#[cfg(feature = "std")]
impl From<ShareMode> for PcscShareMode {
    fn from(mode: ShareMode) -> Self {
        match mode {
            ShareMode::Exclusive => Self::Exclusive,
            ShareMode::Shared => Self::Shared,
            ShareMode::Direct => Self::Direct,
        }
    }
}

/// Transaction mode for card operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionMode {
    /// Start a transaction for each command
    PerCommand,
    /// Only manual transaction management
    Manual,
}

/// Strategy for connecting to a card/reader
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectStrategy {
    /// Connect to a specific reader by name
    #[cfg(any(feature = "std", feature = "alloc"))]
    Reader(String),

    /// Connect to any reader with a card
    AnyCard,

    /// Connect to reader with a card matching this ATR pattern
    #[cfg(any(feature = "std", feature = "alloc"))]
    CardWithAtr(Vec<u8>, Option<Vec<u8>>), // (ATR, mask)

    /// Connect to the first available reader
    FirstAvailable,
}

/// Configuration options for PC/SC transport
#[derive(Debug, Clone)]
pub struct PcscConfig {
    /// Sharing mode for card connections
    pub share_mode: ShareMode,

    /// Preferred protocols for card communication
    #[cfg(feature = "std")]
    pub protocols: PcscProtocols,

    /// Automatically reconnect if the card is reset
    pub auto_reconnect: bool,

    /// Transaction mode
    pub transaction_mode: TransactionMode,
}

#[cfg(feature = "std")]
impl Default for PcscConfig {
    fn default() -> Self {
        Self {
            share_mode: ShareMode::Shared,
            protocols: PcscProtocols::ANY,
            auto_reconnect: true,
            transaction_mode: TransactionMode::PerCommand,
        }
    }
}

#[cfg(not(feature = "std"))]
impl Default for PcscConfig {
    fn default() -> Self {
        Self {
            share_mode: ShareMode::Shared,
            auto_reconnect: true,
            transaction_mode: TransactionMode::PerCommand,
        }
    }
}

impl PcscConfig {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sharing mode
    pub const fn with_share_mode(mut self, mode: ShareMode) -> Self {
        self.share_mode = mode;
        self
    }

    /// Set the preferred protocols
    #[cfg(feature = "std")]
    pub const fn with_protocols(mut self, protocols: PcscProtocols) -> Self {
        self.protocols = protocols;
        self
    }

    /// Set whether to automatically reconnect
    pub const fn with_auto_reconnect(mut self, auto_reconnect: bool) -> Self {
        self.auto_reconnect = auto_reconnect;
        self
    }

    /// Set the transaction mode
    pub const fn with_transaction_mode(mut self, mode: TransactionMode) -> Self {
        self.transaction_mode = mode;
        self
    }
}
