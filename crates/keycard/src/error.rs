use coins_bip39::{MnemonicError, WordlistError};
use iso7816_tlv::TlvError;
use nexum_apdu_core::ApduExecutorErrors;
use nexum_apdu_globalplatform::select::SelectError;

use crate::commands::*;

/// Result type for Keycard operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for Keycard operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Transport-related errors
    #[error(transparent)]
    TransportError(#[from] nexum_apdu_core::transport::TransportError),

    /// Command-related errors
    #[error(transparent)]
    Command(#[from] nexum_apdu_core::command::error::CommandError),

    /// Response-related errors
    #[error(transparent)]
    Response(#[from] nexum_apdu_core::response::error::ResponseError),

    /// Status errors (for status words)
    #[error(transparent)]
    Status(#[from] nexum_apdu_core::response::error::StatusError),

    /// Processor-related errors
    #[error(transparent)]
    Processor(#[from] nexum_apdu_core::processor::ProcessorError),

    /// Secure protocol related errors
    #[error(transparent)]
    SecureProtocol(#[from] nexum_apdu_core::processor::SecureProtocolError),

    /// Secure channel not supported
    #[error("Secure channel not supported")]
    SecureChannelNotSupported,

    #[error("Already initialised")]
    AlreadyInitialised,

    #[error("No available pairing slots")]
    NoAvailablePairingSlots,

    // #[error("Invalid response data")]
    // InvalidResponseData,
    #[error("PIN verification required")]
    PinVerificationRequired,

    #[error("Pairing failed")]
    PairingFailed,

    #[error("Mutual authentication failed")]
    MutualAuthenticationFailed,

    #[error("BIP32 path parsing error")]
    Bip32PathParsingError(coins_bip32::Bip32Error),

    #[error("Invalid derivation path length")]
    InvalidDerivationPathLength,

    #[error("Invalid data")]
    InvalidData(&'static str),

    #[error("Unpad error")]
    UnpadError(#[from] cipher::block_padding::UnpadError),

    #[error("Pad error")]
    PadError(#[from] cipher::inout::PadError),

    #[error("Invalid derivation arguments: {0}")]
    InvalidDerivationArguments(String),

    // Commands
    #[error(transparent)]
    DeriveKeyError(#[from] DeriveKeyError),

    #[error(transparent)]
    ExportKeyError(#[from] ExportKeyError),

    #[error(transparent)]
    FactoryResetError(#[from] FactoryResetError),

    #[error(transparent)]
    GenerateKeyError(#[from] GenerateKeyError),

    #[error(transparent)]
    GenerateMnemonicError(#[from] GenerateMnemonicError),

    #[error(transparent)]
    GetDataError(#[from] GetDataError),

    #[error(transparent)]
    GetStatusError(#[from] GetStatusError),

    #[error(transparent)]
    IdentError(#[from] IdentError),

    #[error(transparent)]
    InitError(#[from] InitError),

    #[error(transparent)]
    LoadKeyError(#[from] LoadKeyError),

    #[error(transparent)]
    MutuallyAuthenticateError(#[from] MutuallyAuthenticateError),

    #[error(transparent)]
    OpenSecureChannelError(#[from] OpenSecureChannelError),

    #[error(transparent)]
    PairError(#[from] PairError),

    #[error(transparent)]
    VerifyPinError(#[from] VerifyPinError),

    #[error(transparent)]
    ChangePinError(#[from] ChangePinError),

    #[error(transparent)]
    UnblockPinError(#[from] UnblockPinError),

    #[error(transparent)]
    RemoveKeyError(#[from] RemoveKeyError),

    #[error(transparent)]
    SelectError(#[from] SelectError),

    #[error(transparent)]
    SetPinlessPathError(#[from] SetPinlessPathError),

    #[error(transparent)]
    SignError(#[from] SignError),

    #[error(transparent)]
    StoreDataError(#[from] StoreDataError),

    #[error(transparent)]
    UnpairError(#[from] UnpairError),

    #[error("TlvError: {0}")]
    TlvError(TlvError),

    #[error(transparent)]
    EllipticCurveError(#[from] k256::elliptic_curve::Error),

    #[error(transparent)]
    EcdsaSignatureError(#[from] k256::ecdsa::Error),

    #[error(transparent)]
    AlloySignatureError(#[from] alloy_signer::Error),

    #[error(transparent)]
    MnemonicError(#[from] MnemonicError),

    #[error(transparent)]
    WordlistError(#[from] WordlistError),

    #[error(transparent)]
    Bip32Error(#[from] coins_bip32::Bip32Error),
}

impl From<TlvError> for Error {
    fn from(error: TlvError) -> Self {
        Error::TlvError(error)
    }
}

impl ApduExecutorErrors for Error {
    type Error = Self;
}
