//! STORE DATA command for GlobalPlatform
//!
//! This command is used to store data on the card, typically for personalization.

use nexum_apdu_macros::apdu_pair;

use crate::constants::{cla, ins, status};

/// STORE DATA command P1 parameter: More blocks
pub const P1_MORE_BLOCKS: u8 = 0x00;
/// STORE DATA command P1 parameter: Last block
pub const P1_LAST_BLOCK: u8 = 0x80;
/// STORE DATA command P1 parameter: DGI format
pub const P1_DGI_FORMAT: u8 = 0x40;

apdu_pair! {
    /// STORE DATA command for GlobalPlatform
    pub struct StoreData {
        command {
            cla: cla::GP,
            ins: ins::STORE_DATA,
            secure: true,

            builders {
                /// Create a STORE DATA command
                pub fn new_with_data(p1: u8, block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(p1, block_number).with_data(data.into())
                }

                /// Create a STORE DATA command for more blocks (not the last one)
                pub fn more_blocks(block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new_with_data(P1_MORE_BLOCKS, block_number, data)
                }

                /// Create a STORE DATA command for the last block
                pub fn last_block(block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new_with_data(P1_LAST_BLOCK, block_number, data)
                }

                /// Create a STORE DATA command with DGI format
                pub fn with_dgi_format(is_last: bool, block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    let p1 = if is_last { P1_LAST_BLOCK | P1_DGI_FORMAT } else { P1_DGI_FORMAT };
                    Self::new_with_data(p1, block_number, data)
                }
            }
        }

        response {
            variants {
                /// Success response (9000)
                #[sw(status::SUCCESS)]
                Success,

                /// Referenced data not found (6A88)
                #[sw(status::REFERENCED_DATA_NOT_FOUND)]
                ReferencedDataNotFound,

                /// Security condition not satisfied (6982)
                #[sw(status::SECURITY_CONDITION_NOT_SATISFIED)]
                SecurityConditionNotSatisfied,

                /// Wrong data (6A80)
                #[sw(status::WRONG_DATA)]
                WrongData,

                /// Other error
                #[sw(_, _)]
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
    fn test_store_data_command() {
        let data = hex!("8401FE0102");
        let cmd = StoreDataCommand::more_blocks(0x00, data.to_vec());

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::STORE_DATA);
        assert_eq!(cmd.p1(), P1_MORE_BLOCKS);
        assert_eq!(cmd.p2(), 0x00);
        assert_eq!(cmd.data(), Some(data.as_ref()));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("80E20000058401FE0102"));
    }

    #[test]
    fn test_store_data_last_block() {
        let data = hex!("8402FE0304");
        let cmd = StoreDataCommand::last_block(0x01, data.to_vec());

        assert_eq!(cmd.p1(), P1_LAST_BLOCK);
        assert_eq!(cmd.p2(), 0x01);

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("80E28001058402FE0304"));
    }

    #[test]
    fn test_store_data_dgi_format() {
        let data = hex!("0101020304");
        let cmd = StoreDataCommand::with_dgi_format(true, 0x02, data.to_vec());

        assert_eq!(cmd.p1(), P1_LAST_BLOCK | P1_DGI_FORMAT);
        assert_eq!(cmd.p2(), 0x02);
    }

    #[test]
    fn test_store_data_response() {
        // Test successful response
        let response_data = hex!("9000");
        let response = StoreDataResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, StoreDataResponse::Success));

        // Test error response
        let response_data = hex!("6A80");
        let response = StoreDataResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, StoreDataResponse::WrongData));
    }
}
