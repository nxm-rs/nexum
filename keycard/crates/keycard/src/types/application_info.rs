use alloy_primitives::hex::{self, ToHexExt};
use iso7816_tlv::ber::{Tag, Tlv, Value};
use std::fmt;

use crate::tags;

use super::{Capabilities, Version, get_primitive_value, signature::PublicKey};

/// Application info return by SELECT command
#[derive(Debug, Clone)]
pub struct ApplicationInfo {
    /// Instance UID (16 bytes)
    pub instance_uid: [u8; 16],
    /// ECC public key (65 bytes or empty)
    pub public_key: Option<k256::PublicKey>,
    /// Application version
    pub version: Version,
    /// Number of remaining pairing slots
    pub remaining_slots: u8,
    /// Key UID (32 bytes SHA-256 hash of master public key or empty)
    pub key_uid: Option<[u8; 32]>,
    /// Supported capabilities
    pub capabilities: Capabilities,
}

impl TryFrom<&Tlv> for ApplicationInfo {
    type Error = crate::Error;

    fn try_from(tlv: &Tlv) -> Result<Self, Self::Error> {
        if tlv.tag() != &Tag::try_from(tags::TEMPLATE_APPLICATION_INFO)? {
            return Err(Self::Error::InvalidData(
                "TLV tag was not application info template tag",
            ));
        }

        match tlv.value() {
            Value::Constructed(tlvs) => {
                let instance_uid: [u8; 16] =
                    get_primitive_value(&Tag::try_from(tags::INSTANCE_UID)?, &tlvs[0])?
                        .try_into()
                        .unwrap();
                let public_key = PublicKey::try_from(&tlvs[1])?;
                let version = Version::try_from(&tlvs[2])?;
                let remaining_slots =
                    get_primitive_value(&Tag::try_from(tags::OTHER).unwrap(), &tlvs[3])?[0];
                let key_uid: Option<[u8; 32]> = {
                    let raw_key_uid =
                        get_primitive_value(&Tag::try_from(tags::KEY_UID).unwrap(), &tlvs[4])?;
                    match raw_key_uid.len() {
                        32 => Some(raw_key_uid.try_into().unwrap()),
                        0 => None,
                        _ => Err(Self::Error::InvalidData("Invalid key UID length"))?,
                    }
                };
                let capabilities = Capabilities::try_from(&tlvs[5])?;

                Ok(Self {
                    instance_uid,
                    public_key: *public_key,
                    version,
                    remaining_slots,
                    key_uid,
                    capabilities,
                })
            }
            _ => Err(Self::Error::InvalidData("TLV value was not constructed")),
        }
    }
}

impl fmt::Display for ApplicationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Application Info:")?;
        writeln!(f, "  Instance UID: {}", hex::encode(self.instance_uid))?;

        writeln!(f, "  Version: {}", self.version)?;
        writeln!(f, "  Remaining pairing slots: {}", self.remaining_slots)?;

        if let Some(ref key_uid) = self.key_uid {
            writeln!(f, "  Key UID: {}", key_uid.encode_hex_with_prefix())?;
        } else {
            writeln!(f, "  Key UID: None (Use GENERATE KEY)")?;
        }

        writeln!(f, "  Capabilities: {}", self.capabilities)?;

        write!(f, "  Secure channel public key: ")?;
        if let Some(ref public_key) = self.public_key {
            write!(f, "{}", public_key.to_sec1_bytes().encode_hex_with_prefix())
        } else {
            write!(f, "None")
        }
    }
}
