pub const KEYCARD_AID: &[u8] = b"\xA0\x00\x00\x08\x04\x00\x01\x01";
pub const CASHCARD_AID: &[u8] = b"\xA0\x00\x00\x08\x04\x00\x01\x03";

pub mod tags {
    /// Signature template containing:
    /// - TAG_ECC_PUBLIC_KEY
    /// - TAG_ECDSA_SIGNATURE
    pub const TEMPLATE_SIGNATURE: u8 = 0xA0;
    /// Keypair template containing:
    /// - TAG_ECC_PRIVATE_KEY
    /// - optional TAG_CHAIN_CODE / TAG_ECC_PUBLIC_KEY
    pub const TEMPLATE_KEYPAIR: u8 = 0xA1;
    /// Application status template containing:
    /// - TAG_OTHER (for 'PIN' and 'PUK' retry count)
    /// - TAG_KEY_INITIALIZED
    pub const TEMPLATE_APPLICATION_STATUS: u8 = 0xA3;
    /// Application info template containing:
    /// - TAG_INSTANCE_UID
    /// - TAG_ECC_PUBLIC_KEY
    /// - TAG_OTHER (application version and number remaining pairing slots)
    /// - TAG_KEY_UID
    /// - TAG_CAPABILITIES
    pub const TEMPLATE_APPLICATION_INFO: u8 = 0xA4;

    /// Instance UID (16 bytes)
    pub const INSTANCE_UID: u8 = 0x8F;
    /// ECC Public Key (Uncompressed, ie. 65 bytes, or 0 bytes if not available)
    pub const ECC_PUBLIC_KEY: u8 = 0x80;
    /// ECC Private key (32 bytes)
    pub const ECC_PRIVATE_KEY: u8 = 0x81;
    /// Chain code
    pub const CHAIN_CODE: u8 = 0x82;
    /// Application version (2 bytes) / number of remaining pairing slots (1 byte)
    pub const OTHER: u8 = 0x02;
    /// Key UID (32 bytes)
    pub const KEY_UID: u8 = 0x8E;
    /// Keycard capabilities (1 byte)
    pub const CAPABILITIES: u8 = 0x8D;
    /// Certificate
    pub const CERTIFICATE: u8 = 0x8A;
    /// ECDSA signature (then contains an array of TAG_OTHER for the 'R' and 'S' values)
    pub const ECDSA_SIGNATURE: u8 = 0x30;

    /// Key initialized (0xff if key is initialized, 0 otherwise)
    pub const KEY_INITIALIZED: u8 = 0x01;
}
