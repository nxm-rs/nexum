mod application;
mod commands;
mod constants;
mod crypto;
mod error;
mod secrets;
mod secure_channel;
mod session;
mod types;
mod validation;

pub use application::{CredentialType, Keycard};
pub use commands::*;
pub use constants::*;
pub use crypto::Challenge;
pub use error::{CoreResultExt, Error, GpResultExt, Result, ResultExt};
pub use secrets::Secrets;
pub use secure_channel::KeycardSecureChannel;
pub use types::{ApplicationInfo, ApplicationStatus, Keypair, PairingInfo, PublicKey};

pub use nexum_apdu_core::prelude::CardExecutor;
pub use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscTransport};

/// Represents the version of the applet protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppletVersion {
    /// Versions before 3.1
    Legacy,
    /// Version 3.1 and above
    V3_1,
}

/// Create a Keycard instance AID with the specified index
pub fn keycard_instance_aid(index: u8) -> Vec<u8> {
    assert!(index >= 1);
    let mut aid = Vec::from(KEYCARD_AID);
    aid.push(index);
    aid
}
