use bytes::Bytes;
use iso7816_tlv::ber::{Tag, Tlv, Value};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use k256::{PublicKey, SecretKey};
use nexum_apdu_globalplatform::constants::status;
use nexum_apdu_macros::apdu_pair;

use super::CLA_GP;

apdu_pair! {
    /// LOAD KEY command for Keycard
    pub struct LoadKey {
        command {
            cla: CLA_GP,
            ins: 0xD0,
            required_security_level: SecurityLevel::authenticated_encrypted(),

            builders {
                /// Create a LOAD KEY command for loading an ECC secp256k1 keypair
                pub fn load_keypair(public_key: Option<PublicKey>, private_key: SecretKey) -> Result<Self, crate::Error> {
                    let buf = Bytes::from(
                        create_keypair_template(
                            public_key,
                            private_key,
                            None
                        )?
                        .to_vec());

                    Ok(Self::new(0x01, 0x00).with_data(buf).with_le(0))
                }

                /// Create a LOAD KEY command for loading an ECC secp256k1 extended keypair
                pub fn load_extended_keypair(public_key: Option<PublicKey>, private_key: SecretKey, chain_code: [u8; 32]) -> Result<Self, crate::Error> {
                    let buf = Bytes::from(
                        create_keypair_template(
                            public_key,
                            private_key,
                            Some(chain_code)
                        )?
                        .to_vec()
                    );

                    Ok(Self::new(0x02, 0x00).with_data(buf).with_le(0))
                }

                /// Create a LOAD KEY command for loading a BIP39 seed
                pub fn load_bip39_seed(seed: &[u8; 64]) -> Self {
                    Self::new(0x03, 0x00).with_data(Bytes::copy_from_slice(seed)).with_le(0)
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success {
                    /// Key UID
                    key_uid: [u8; 32],
                }
            }

            errors {
                /// Wrong data: format is invalid
                #[sw(status::SW_WRONG_DATA)]
                #[error("Wrong data: format is invalid")]
                WrongData,

                /// Incorrect P1/P2: P1 is invalid
                #[sw(status::SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: P1 is invalid")]
                IncorrectP1P2,
            }

            custom_parse = |response: &nexum_apdu_core::Response| -> Result<LoadKeyOk, LoadKeyError> {
                use nexum_apdu_core::ApduResponse;

                match response.status() {
                    status::SW_NO_ERROR => {
                        match response.payload() {
                            Some(payload) => Ok(LoadKeyOk::Success {
                                key_uid: payload.to_vec().try_into()
                                    .map_err(|_| LoadKeyError::WrongData)?,
                            }),
                            None => Err(LoadKeyError::WrongData),
                        }
                    },
                    status::SW_WRONG_DATA => Err(LoadKeyError::WrongData),
                    status::SW_INCORRECT_P1P2 => Err(LoadKeyError::IncorrectP1P2),
                    _ => Err(LoadKeyError::Unknown{ sw1: response.status().sw1, sw2: response.status().sw2 }),
                }
            }
        }
    }
}

pub const TAG_KEYPAIR_TEMPLATE: u8 = 0xA1;
pub const TAG_ECC_PUBLIC_KEY: u8 = 0x80;
pub const TAG_ECC_PRIVATE_KEY: u8 = 0x81;
pub const TAG_CHAIN_CODE: u8 = 0x82;

fn create_keypair_template(
    public_key: Option<PublicKey>,
    private_key: SecretKey,
    chain_code: Option<[u8; 32]>,
) -> Result<Tlv, crate::Error> {
    Tlv::new(
        Tag::try_from(TAG_KEYPAIR_TEMPLATE)?,
        Value::Constructed({
            let mut tlvs: Vec<Tlv> = vec![];
            tlvs.push(Tlv::new(
                Tag::try_from(TAG_ECC_PRIVATE_KEY)?,
                Value::Primitive(private_key.to_bytes().as_slice().to_vec()),
            )?);
            if let Some(public_key) = public_key {
                tlvs.push(Tlv::new(
                    Tag::try_from(TAG_ECC_PUBLIC_KEY)?,
                    Value::Primitive(public_key.to_encoded_point(false).as_bytes().to_vec()),
                )?);
            }
            if let Some(chain_code) = chain_code {
                tlvs.push(Tlv::new(
                    Tag::try_from(TAG_CHAIN_CODE)?,
                    Value::Primitive(chain_code.to_vec()),
                )?);
            }

            tlvs
        }),
    )
    .map_err(Into::into)
}
