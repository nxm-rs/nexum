/// Pairing information structure
#[derive(Debug, Clone)]
pub struct PairingInfo {
    pub key: [u8; 32],
    pub index: u8,
}
