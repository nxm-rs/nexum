use cipher::Key;

use crate::crypto::KeycardScp;

/// Pairing information structure
#[derive(Debug, Clone)]
pub struct PairingInfo {
    pub key: Key<KeycardScp>,
    pub index: u8,
}
