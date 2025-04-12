//! STORE DATA command for GlobalPlatform
//!
//! This command is used to store data on the card, typically for personalization.

use nexum_apdu_macros::apdu_pair;

use crate::constants::*;

apdu_pair! {
    /// STORE DATA command for GlobalPlatform
    pub struct StoreData {
        command {
            cla: cla::GP,
            ins: ins::STORE_DATA,
            required_security_level: SecurityLevel::mac_protected(),

            builders {
                /// Create a STORE DATA command
                pub fn new_with_data(p1: u8, block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(p1, block_number).with_data(data.into())
                }

                /// Create a STORE DATA command for more blocks (not the last one)
                pub fn more_blocks(block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new_with_data(0x00, block_number, data)
                }

                /// Create a STORE DATA command for the last block
                pub fn last_block(block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new_with_data(0x80, block_number, data)
                }

                /// Create a STORE DATA command with DGI format
                pub fn with_dgi_format(is_last: bool, block_number: u8, data: impl Into<bytes::Bytes>) -> Self {
                    let p1 = if is_last { 0x80 | 0x40 } else { 0x40 };
                    Self::new_with_data(p1, block_number, data)
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
                /// Referenced data not found
                #[sw(SW_REFERENCED_DATA_NOT_FOUND)]
                #[error("Referenced data not found")]
                ReferencedDataNotFound,

                /// Security status not satisfied
                #[sw(SW_SECURITY_STATUS_NOT_SATISFIED)]
                #[error("Security status not satisfied")]
                SecurityStatusNotSatisfied,

                /// Wrong data
                #[sw(SW_WRONG_DATA)]
                #[error("Wrong data")]
                WrongData,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hex_literal::hex;
    use nexum_apdu_core::ApduCommand;

    #[test]
    fn test_store_data_command() {
        let data = hex!("8401FE0102");
        let cmd = StoreDataCommand::more_blocks(0x00, data.to_vec());

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::STORE_DATA);
        assert_eq!(cmd.p1(), 0x00);
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

        assert_eq!(cmd.p1(), 0x80);
        assert_eq!(cmd.p2(), 0x01);

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("80E28001058402FE0304"));
    }

    #[test]
    fn test_store_data_dgi_format() {
        let data = hex!("0101020304");
        let cmd = StoreDataCommand::with_dgi_format(true, 0x02, data.to_vec());

        assert_eq!(cmd.p1(), 0x80 | 0x40);
        assert_eq!(cmd.p2(), 0x02);
    }

    #[test]
    fn test_store_data_response() {
        // Test successful response
        let response_data = Bytes::from_static(&hex!("9000"));
        let result = StoreDataCommand::parse_response_raw(response_data).unwrap();
        assert!(matches!(result, StoreDataOk::Success));

        // Test error response
        let response_data = Bytes::from_static(&hex!("6A80"));
        let result = StoreDataCommand::parse_response_raw(response_data).unwrap_err();
        assert!(matches!(result, StoreDataError::WrongData));
    }
}
