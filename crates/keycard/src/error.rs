//! Error types for Keycard operations
//!
//! This module provides error types specific to Keycard operations.
//! It centralizes all error variants to simplify error handling and
//! facilitate better error propagation throughout the codebase.

use coins_bip39::{MnemonicError, WordlistError};
use iso7816_tlv::TlvError;
use thiserror::Error;

use crate::commands::*;

/// Result type for Keycard operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for Keycard operations
///
/// This enum represents all possible errors that can occur during Keycard
/// operations, including communication errors, cryptographic errors,
/// and specific command errors returned by the card.
#[derive(Debug, Error)]
pub enum Error {
    //
    // Core and external dependency errors
    //
    /// Core error from nexum_apdu_core
    #[error(transparent)]
    Core(#[from] nexum_apdu_core::Error),

    /// GlobalPlatform error
    #[error(transparent)]
    GlobalPlatform(#[from] nexum_apdu_globalplatform::Error),

    /// TLV encoding/decoding error
    #[error("TLV error: {0}")]
    Tlv(TlvError),

    /// BIP39 mnemonic error
    #[error(transparent)]
    Mnemonic(#[from] MnemonicError),

    /// BIP39 wordlist error
    #[error(transparent)]
    Wordlist(#[from] WordlistError),

    /// BIP32 derivation error
    #[error(transparent)]
    Bip32(#[from] coins_bip32::Bip32Error),

    //
    // Cryptographic errors
    //
    /// Elliptic curve cryptography error
    #[error(transparent)]
    EllipticCurve(#[from] k256::elliptic_curve::Error),

    /// ECDSA signature error
    #[error(transparent)]
    EcdsaSignature(#[from] k256::ecdsa::Error),

    /// Alloy signature error
    #[error(transparent)]
    AlloySignature(#[from] alloy_signer::Error),

    /// Padding error when encrypting
    #[error("Padding error when encrypting")]
    PadError(#[from] cipher::inout::PadError),

    /// Unpadding error when decrypting
    #[error("Unpadding error when decrypting")]
    UnpadError(#[from] cipher::block_padding::UnpadError),

    //
    // Keycard-specific errors
    //
    /// Capability not supported
    #[error("Capability not supported: {0}")]
    CapabilityNotSupported(&'static str),

    /// Secure channel not supported
    #[error("Secure channel not supported")]
    SecureChannelNotSupported,

    /// Card is already initialized
    #[error("Card is already initialized")]
    AlreadyInitialized,

    /// No available pairing slots on the card
    #[error("No available pairing slots")]
    NoAvailablePairingSlots,

    /// PIN verification required for this operation
    #[error("PIN verification required")]
    PinVerificationRequired,

    /// Pairing with the card failed
    #[error("Pairing failed")]
    PairingFailed,

    /// Pairing information required
    #[error("Pairing information required")]
    PairingRequired,

    /// Mutual authentication failed
    #[error("Mutual authentication failed")]
    MutualAuthenticationFailed,

    /// BIP32 path parsing error
    #[error("BIP32 path parsing error: {0}")]
    Bip32PathParsingError(coins_bip32::Bip32Error),

    /// Invalid derivation path length
    #[error("Invalid derivation path length")]
    InvalidDerivationPathLength,

    /// Invalid data format
    #[error("Invalid data: {0}")]
    InvalidData(&'static str),

    /// Invalid arguments for key derivation
    #[error("Invalid derivation arguments: {0}")]
    InvalidDerivationArguments(String),

    /// Operation cancelled by user
    #[error("Operation cancelled by user")]
    UserCancelled,

    //
    // Command-specific errors
    //
    /// Error from DERIVE KEY command
    #[error(transparent)]
    DeriveKeyError(#[from] DeriveKeyError),

    /// Error from EXPORT KEY command
    #[error(transparent)]
    ExportKeyError(#[from] ExportKeyError),

    /// Error from FACTORY RESET command
    #[error(transparent)]
    FactoryResetError(#[from] FactoryResetError),

    /// Error from GENERATE KEY command
    #[error(transparent)]
    GenerateKeyError(#[from] GenerateKeyError),

    /// Error from GENERATE MNEMONIC command
    #[error(transparent)]
    GenerateMnemonicError(#[from] GenerateMnemonicError),

    /// Error from GET DATA command
    #[error(transparent)]
    GetDataError(#[from] GetDataError),

    /// Error from GET STATUS command
    #[error(transparent)]
    GetStatusError(#[from] GetStatusError),

    /// Error from IDENT command
    #[error(transparent)]
    IdentError(#[from] IdentError),

    /// Error from INIT command
    #[error(transparent)]
    InitError(#[from] InitError),

    /// Error from LOAD KEY command
    #[error(transparent)]
    LoadKeyError(#[from] LoadKeyError),

    /// Error from MUTUALLY AUTHENTICATE command
    #[error(transparent)]
    MutuallyAuthenticateError(#[from] MutuallyAuthenticateError),

    /// Error from OPEN SECURE CHANNEL command
    #[error(transparent)]
    OpenSecureChannelError(#[from] OpenSecureChannelError),

    /// Error from PAIR command
    #[error(transparent)]
    PairError(#[from] PairError),

    /// Error from VERIFY PIN command
    #[error(transparent)]
    VerifyPinError(#[from] VerifyPinError),

    /// Error from CHANGE PIN command
    #[error(transparent)]
    ChangePinError(#[from] ChangePinError),

    /// Error from UNBLOCK PIN command
    #[error(transparent)]
    UnblockPinError(#[from] UnblockPinError),

    /// Error from REMOVE KEY command
    #[error(transparent)]
    RemoveKeyError(#[from] RemoveKeyError),

    /// Error from SELECT command
    #[error(transparent)]
    SelectError(#[from] nexum_apdu_globalplatform::select::SelectError),

    /// Error from SET PINLESS PATH command
    #[error(transparent)]
    SetPinlessPathError(#[from] SetPinlessPathError),

    /// Error from SIGN command
    #[error(transparent)]
    SignError(#[from] SignError),

    /// Error from STORE DATA command
    #[error(transparent)]
    StoreDataError(#[from] StoreDataError),

    /// Error from UNPAIR command
    #[error(transparent)]
    UnpairError(#[from] UnpairError),

    //
    // General error handling
    //
    /// Context with source error
    #[error("{context}: {source}")]
    Context {
        /// Contextual message
        context: String,
        /// Source error
        source: Box<Self>,
    },

    /// Other error with static message
    #[error("{0}")]
    Other(&'static str),

    /// Other error with dynamic message
    #[error("{0}")]
    Message(String),
}

impl Error {
    /// Create a new error with context information
    pub fn with_context<S: Into<String>>(self, context: S) -> Self {
        Self::Context {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Create a new error with a static message
    pub const fn other(message: &'static str) -> Self {
        Self::Other(message)
    }

    /// Create a new error with a dynamic message
    pub fn message<S: Into<String>>(message: S) -> Self {
        Self::Message(message.into())
    }

    /// Create a new invalid data error
    pub const fn invalid_data(message: &'static str) -> Self {
        Self::InvalidData(message)
    }
}

/// Extension trait for Result with context addition
pub trait ResultExt<T> {
    /// Add context to an error
    fn context<S: Into<String>>(self, context: S) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context<S: Into<String>>(self, context: S) -> Self {
        self.map_err(|e| e.with_context(context))
    }
}

/// Extension trait for nexum_apdu_core::Result
pub trait CoreResultExt<T> {
    /// Convert core result to Keycard result
    fn to_keycard(self) -> Result<T>;
}

impl<T> CoreResultExt<T> for std::result::Result<T, nexum_apdu_core::Error> {
    fn to_keycard(self) -> Result<T> {
        self.map_err(Error::from)
    }
}

/// Extension trait for nexum_apdu_globalplatform::Result
pub trait GpResultExt<T> {
    /// Convert GlobalPlatform result to Keycard result
    fn to_keycard(self) -> Result<T>;
}

impl<T> GpResultExt<T> for std::result::Result<T, nexum_apdu_globalplatform::Error> {
    fn to_keycard(self) -> Result<T> {
        self.map_err(Error::from)
    }
}

impl From<TlvError> for Error {
    fn from(error: TlvError) -> Self {
        Error::Tlv(error)
    }
}
