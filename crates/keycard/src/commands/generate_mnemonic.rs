use coins_bip39::Mnemonic;
use nexum_apdu_globalplatform::constants::status;
use nexum_apdu_macros::apdu_pair;

use crate::Error;

use super::CLA_GP;

apdu_pair! {
    /// GENERATE MNEMONIC command for Keycard
    pub struct GenerateMnemonic {
        command {
            cla: CLA_GP,
            ins: 0xD2,
            required_security_level: SecurityLevel::encrypted(),

            builders {
                /// Create a GENERATE MNEMONIC command with a given number of words (12, 15, 18, 21, 24)
                pub fn with_words(words: u8) -> Result<Self, GenerateMnemonicError> {
                    match words {
                        12 | 15 | 18 | 21 | 24 => Ok(Self::new(words / 3, 0x00).with_le(0)),
                        _ => Err(GenerateMnemonicError::IncorrectChecksumSize),
                    }
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success {
                    seed: Vec<u8>
                }
            }

            errors {
                /// Incorrect P1/P2: Checksum is out of range (between 4 and 8)
                #[sw(status::SW_INCORRECT_P1P2)]
                #[error("Incorrect P1/P2: Checksum is out of range (between 4 and 8)")]
                IncorrectChecksumSize,
            }
        }
    }
}

impl GenerateMnemonicOk {
    pub fn to_phrase<L>(&self) -> Result<Mnemonic<L>, Error>
    where
        L: coins_bip39::Wordlist,
    {
        match self {
            Self::Success { seed } => {
                let mut words = Vec::new();

                for chunk in seed.chunks_exact(2) {
                    if chunk.len() == 2 {
                        let index = u16::from_be_bytes([chunk[0], chunk[1]]) as usize;
                        words.push(L::get(index)?);
                    }
                }

                Mnemonic::new_from_phrase(words.join(" ").as_str()).map_err(Into::into)
            }
        }
    }
}
