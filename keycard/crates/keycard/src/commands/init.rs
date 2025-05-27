use bytes::BytesMut;
use nexum_apdu_globalplatform::constants::status::*;
use nexum_apdu_macros::apdu_pair;

use crate::{
    crypto::{generate_ecdh_shared_secret, one_shot_encrypt},
    secrets::Secrets,
};

use super::CLA_GP;

apdu_pair! {
    /// INIT command for Keycard
    pub struct Init {
        command {
            cla: CLA_GP,
            ins: 0xFE,

            builders {
                /// Create an INIT command with parameters
                pub fn with_card_pubkey_and_secrets(card_public_key: k256::PublicKey, secrets: &Secrets) -> Self {
                    // Generate an ephemeral keypair, only needed for ECDH
                    let host_private_key = k256::SecretKey::random(&mut rand_v8::thread_rng());
                    let shared_secret = generate_ecdh_shared_secret(
                        &host_private_key,
                        &card_public_key
                    );
                    let mut data = BytesMut::from(secrets.to_bytes());

                    Self::new(0x00, 0x00)
                        .with_data(one_shot_encrypt(
                            &host_private_key.public_key(),
                            &shared_secret,
                            &mut data))
                        .with_le(0)
                }

                /// Create an INIT command and automatically generate secrets
                pub fn with_card_pubkey(card_public_key: k256::PublicKey) -> Self {
                    Self::with_card_pubkey_and_secrets(
                        card_public_key,
                        &Secrets::generate_v3_1(3, 5, true)
                    )
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(SW_NO_ERROR)]
                Success,
            }

            errors {
                /// INS not supported
                #[sw(SW_INS_NOT_SUPPORTED)]
                #[error("Already initialized")]
                AlreadyInitialized,

                /// Wrong data
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data")]
                WrongData,
            }
        }
    }
}
