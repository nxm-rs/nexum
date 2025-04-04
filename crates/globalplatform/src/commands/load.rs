//! LOAD command for GlobalPlatform
//!
//! This command is used to load executable code (CAP files) to the card.

use nexum_apdu_macros::apdu_pair;

use crate::constants::{cla, ins, load_p1, status};

apdu_pair! {
    /// LOAD command for GlobalPlatform
    pub struct Load {
        command {
            cla: cla::GP,
            ins: ins::LOAD,
            required_security_level: SecurityLevel::mac_protected(),

            builders {
                /// Create a LOAD command with block data
                pub fn with_block_data(p1: u8, block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(p1, block_number).with_data(data.into()).with_le(0)
                }

                /// Create a LOAD command for more blocks
                pub fn more_blocks(block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::with_block_data(load_p1::MORE_BLOCKS, block_number, data.into())
                }

                /// Create a LOAD command for the last block
                pub fn last_block(block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::with_block_data(load_p1::LAST_BLOCK, block_number, data.into())
                }
            }
        }

        response {
            ok {
                /// Success response
                #[sw(status::SW_NO_ERROR)]
                Success,
            }

            errors {
                /// Security status not satisfied
                #[sw(status::SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied")]
                SecurityStatusNotSatisfied,

                /// Wrong length
                #[sw(status::SW_WRONG_LENGTH)]
                #[error("Wrong length")]
                WrongLength,

                /// Other error
                #[sw(_, _)]
                #[error("Other error")]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use nexum_apdu_core::ApduCommand;

    #[test]
    fn test_load_command() {
        let block_data = hex!("C4020304");
        let cmd = LoadCommand::more_blocks(0x01, block_data.to_vec());

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::LOAD);
        assert_eq!(cmd.p1(), load_p1::MORE_BLOCKS);
        assert_eq!(cmd.p2(), 0x01);
        assert_eq!(cmd.data(), Some(block_data.as_ref()));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("80E8000104C402030400"));
    }

    #[test]
    fn test_load_last_block() {
        let block_data = hex!("C4020304");
        let cmd = LoadCommand::last_block(0x02, block_data.to_vec());

        assert_eq!(cmd.p1(), load_p1::LAST_BLOCK);
        assert_eq!(cmd.p2(), 0x02);
    }

    #[test]
    fn test_load_response() {
        // Test successful response
        let response_data = hex!("9000");
        let response = LoadResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, LoadResponse::Success));

        // Test error response
        let response_data = hex!("6982");
        let response = LoadResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, LoadResponse::SecurityStatusNotSatisfied));
    }
}
