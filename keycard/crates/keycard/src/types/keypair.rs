use std::fmt;

use iso7816_tlv::{
    TlvError,
    ber::{Tag, Tlv, Value},
};
use k256::{PublicKey, SecretKey};

// Import the tags from the constants module
use crate::tags;

use super::get_primitive_value;

/// Represents a keypair template (tag 0xA1) that can be used for both loading and exporting keys.
///
/// For EXPORT KEY command, this struct is obtained by parsing the response.
/// For LOAD KEY command, this struct can be created and serialized to send to the card.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Keypair {
    /// ECC public key component (tag 0x80)
    pub public_key: Option<PublicKey>,

    /// ECC private key component (tag 0x81)
    pub private_key: Option<SecretKey>,

    /// Chain code for extended keys (tag 0x82)
    pub chain_code: Option<Vec<u8>>,
}

impl Keypair {
    /// Creates a keypair with a private key for loading to the card
    ///
    /// This is primarily used for the LOAD KEY command.
    /// Note that the public key component is optional when loading a key.
    pub fn with_private_key(private_key: SecretKey) -> Self {
        Self {
            private_key: Some(private_key),
            ..Default::default()
        }
    }

    /// Creates a keypair with public and private keys for loading to the card
    ///
    /// This is primarily used for the LOAD KEY command.
    pub fn with_keypair(public_key: PublicKey, private_key: SecretKey) -> Self {
        Self {
            public_key: Some(public_key),
            private_key: Some(private_key),
            ..Default::default()
        }
    }

    /// Creates an extended keypair with public key, private key, and chain code for loading to the card
    ///
    /// This is primarily used for the LOAD KEY command with P1=0x02 (extended keypair).
    pub fn with_extended_keypair(
        public_key: PublicKey,
        private_key: SecretKey,
        chain_code: Vec<u8>,
    ) -> Self {
        Self {
            public_key: Some(public_key),
            private_key: Some(private_key),
            chain_code: Some(chain_code),
        }
    }

    /// Determines if this keypair has a chain code, making it an extended keypair
    pub fn is_extended(&self) -> bool {
        self.chain_code.is_some()
    }

    /// Serialize the keypair to bytes for sending to the card
    ///
    /// This is used for the LOAD KEY command.
    pub fn to_bytes(&self) -> Result<Vec<u8>, crate::Error> {
        let tlv: Tlv = self.try_into()?;
        Ok(tlv.to_vec())
    }
}

impl TryFrom<Tlv> for Keypair {
    type Error = crate::Error;

    fn try_from(tlv: Tlv) -> Result<Self, Self::Error> {
        if tlv.tag() != &Tag::try_from(tags::TEMPLATE_KEYPAIR)? {
            return Err(Self::Error::InvalidData(
                "TLV tag was not keypair template tag",
            ));
        }

        match tlv.value() {
            Value::Primitive(_) => Err(Self::Error::InvalidData(
                "Expected constructed TLV for keypair template",
            )),
            Value::Constructed(tlvs) => {
                let mut keypair = Keypair::default();
                for tlv in tlvs {
                    let tag = tlv.tag();

                    if tag == &Tag::try_from(tags::ECC_PUBLIC_KEY)? {
                        keypair.public_key =
                            Some(PublicKey::from_sec1_bytes(&get_primitive_value(tag, tlv)?)?);
                    } else if tag == &Tag::try_from(tags::ECC_PRIVATE_KEY)? {
                        keypair.private_key =
                            Some(SecretKey::from_slice(&get_primitive_value(tag, tlv)?)?);
                    } else if tag == &Tag::try_from(tags::CHAIN_CODE)? {
                        keypair.chain_code = Some(get_primitive_value(tag, tlv)?);
                    }
                }
                Ok(keypair)
            }
        }
    }
}

impl TryFrom<&[u8]> for Keypair {
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (tlv, _) = Tlv::parse(value);
        Self::try_from(tlv?)
    }
}

impl TryInto<Tlv> for &Keypair {
    type Error = crate::Error;

    fn try_into(self) -> Result<Tlv, Self::Error> {
        let template_tag = Tag::try_from(tags::TEMPLATE_KEYPAIR)?;
        let mut inner_tlvs = Vec::new();

        // Helper function to create TLV for each component
        let add_tlv =
            |tag_value: u8, data: &Option<Vec<u8>>, tlvs: &mut Vec<Tlv>| -> Result<(), TlvError> {
                if let Some(data) = data {
                    let tag = Tag::try_from(tag_value)?;
                    let tlv = Tlv::new(tag, Value::Primitive(data.clone()))?;
                    tlvs.push(tlv);
                }
                Ok(())
            };

        // Add TLV for each component if present
        add_tlv(
            tags::ECC_PUBLIC_KEY,
            &self.public_key.map(|f| f.to_sec1_bytes().to_vec()),
            &mut inner_tlvs,
        )?;
        add_tlv(
            tags::ECC_PRIVATE_KEY,
            &self.private_key.as_ref().map(|f| f.to_bytes().to_vec()),
            &mut inner_tlvs,
        )?;
        add_tlv(tags::CHAIN_CODE, &self.chain_code, &mut inner_tlvs)?;

        Ok(Tlv::new(template_tag, Value::Constructed(inner_tlvs))?)
    }
}

// For security, don't display private key in debug output
impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field(
                "public_key",
                &self
                    .public_key
                    .as_ref()
                    .map(|pk| format!("[Public Key: {} bytes]", pk.to_sec1_bytes().len())),
            )
            .field(
                "private_key",
                &self.private_key.as_ref().map(|_| "[Private Key Present]"),
            )
            .field(
                "chain_code",
                &self.chain_code.as_ref().map(|_| "[Chain Code Present]"),
            )
            .finish()
    }
}

impl fmt::Display for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Keypair:")?;

        match &self.public_key {
            Some(pk) => writeln!(f, "  Public Key: {} bytes", pk.to_sec1_bytes().len())?,
            None => writeln!(f, "  Public Key: Not present")?,
        }

        match &self.private_key {
            Some(_) => writeln!(f, "  Private Key: Present")?,
            None => writeln!(f, "  Private Key: Not present")?,
        }

        match &self.chain_code {
            Some(_) => writeln!(f, "  Chain Code: Present (Extended keypair)")?,
            None => writeln!(f, "  Chain Code: Not present")?,
        }

        Ok(())
    }
}
