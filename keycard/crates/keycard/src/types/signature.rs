use derive_more::{AsRef, Deref};
use iso7816_tlv::ber::{Tag, Tlv, Value};
use k256::ecdsa;

use crate::tags;

use super::get_primitive_value;

#[derive(Debug, Clone, PartialEq, Eq, AsRef)]
pub struct Signature {
    pub public_key: k256::PublicKey,
    pub signature: EcdsaSignature,
}

impl TryFrom<Tlv> for Signature {
    type Error = crate::Error;

    fn try_from(tlv: Tlv) -> Result<Self, Self::Error> {
        if tlv.tag() != &Tag::try_from(tags::TEMPLATE_SIGNATURE)? {
            return Err(Self::Error::InvalidData(
                "TLV tag was not signature template tag",
            ));
        }

        match tlv.value() {
            Value::Primitive(_) => Err(Self::Error::InvalidData(
                "Expected constructed TLV for signature template",
            )),
            Value::Constructed(tlvs) => {
                let public_key = PublicKey::try_from(&tlvs[0])?;
                let signature = EcdsaSignature::try_from(&tlvs[1])?;
                Ok(Signature {
                    public_key: public_key
                        .as_ref()
                        .ok_or(Self::Error::InvalidData("Invalid public key"))?,
                    signature,
                })
            }
        }
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let (tlv, _) = Tlv::parse(value);
        Self::try_from(tlv?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, AsRef, Deref)]
pub struct PublicKey(Option<k256::PublicKey>);

impl TryFrom<&Tlv> for PublicKey {
    type Error = crate::Error;

    fn try_from(tlv: &Tlv) -> Result<Self, Self::Error> {
        if tlv.tag() != &Tag::try_from(tags::ECC_PUBLIC_KEY)? {
            return Err(Self::Error::InvalidData("Invalid tag"));
        }

        let public_key = {
            let value = get_primitive_value(&Tag::try_from(tags::ECC_PUBLIC_KEY)?, tlv)?;
            match value.len() {
                0 => None,
                65 => Some(k256::PublicKey::from_sec1_bytes(value.as_slice())?),
                _ => return Err(Self::Error::InvalidData("Invalid public key length")),
            }
        };

        Ok(PublicKey(public_key))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, AsRef, Deref)]
pub struct EcdsaSignature(k256::ecdsa::Signature);

impl TryFrom<&Tlv> for EcdsaSignature {
    type Error = crate::Error;

    fn try_from(tlv: &Tlv) -> Result<Self, Self::Error> {
        if tlv.tag() != &Tag::try_from(tags::ECDSA_SIGNATURE)? {
            return Err(Self::Error::InvalidData("Invalid tag"));
        }

        match tlv.value() {
            Value::Primitive(_) => Err(Self::Error::InvalidData(
                "Expected constructed TLV for signature template",
            )),
            Value::Constructed(tlvs) => {
                let r_raw = get_primitive_value(&Tag::try_from(tags::OTHER)?, &tlvs[0])?;
                let r: [u8; 32] = r_raw[r_raw.len() - 32..]
                    .try_into()
                    .map_err(|_| Self::Error::InvalidData("Invalid r length"))?;
                let s_raw = get_primitive_value(&Tag::try_from(tags::OTHER)?, &tlvs[1])?;
                let s: [u8; 32] = s_raw[s_raw.len() - 32..]
                    .try_into()
                    .map_err(|_| Self::Error::InvalidData("Invalid s length"))?;
                Ok(EcdsaSignature(ecdsa::Signature::from_scalars(r, s)?))
            }
        }
    }
}
