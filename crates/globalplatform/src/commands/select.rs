//! SELECT command for GlobalPlatform
//!
//! This command is used to select an application or file by its AID.

use apdu_macros::apdu_pair;

use crate::constants::{cla, ins, select_p1, status};

apdu_pair! {
    /// SELECT command for GlobalPlatform
    pub struct Select {
        command {
            cla: cla::ISO7816,
            ins: ins::SELECT,
            secure: false,

            builders {
                /// Create a new SELECT command with AID
                pub fn with_aid(aid: impl Into<bytes::Bytes>) -> Self {
                    Self::new_with_params(select_p1::BY_NAME, 0x00, aid.into())
                }

                /// Create a new SELECT command with specific P1, P2, and AID
                pub fn new_with_params(p1: u8, p2: u8, aid: impl Into<bytes::Bytes>) -> Self {
                    let mut cmd = Self::new(p1, p2).with_data(aid.into());
                    cmd = cmd.with_le(0x00);
                    cmd
                }
            }
        }

        response {
            variants {
                /// Success response (9000)
                #[sw(status::SUCCESS)]
                Success {
                    fci: Option<Vec<u8>>,
                },

                /// File or application not found (6A82)
                #[sw(status::FILE_NOT_FOUND)]
                NotFound,

                /// Security condition not satisfied (6982)
                #[sw(status::SECURITY_CONDITION_NOT_SATISFIED)]
                SecurityConditionNotSatisfied,

                /// Incorrect parameters (6A86)
                #[sw(status::COMMAND_NOT_ALLOWED)]
                IncorrectParameters,

                /// Other error
                #[sw(_, _)]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            parse_payload = |payload: &[u8], _sw: apdu_core::StatusWord, variant: &mut Self| -> Result<(), apdu_core::Error> {
                if let Self::Success { fci } = variant {
                    if !payload.is_empty() {
                        *fci = Some(payload.to_vec());
                    }
                }
                Ok(())
            }

            methods {
                /// Returns true if the selection was successful
                pub fn is_success(&self) -> bool {
                    matches!(self, Self::Success { .. })
                }

                /// Returns true if the file or application was not found
                pub fn is_not_found(&self) -> bool {
                    matches!(self, Self::NotFound { .. })
                }

                /// Get the File Control Information if available
                pub fn fci(&self) -> Option<&[u8]> {
                    match self {
                        Self::Success { fci } => fci.as_deref(),
                        _ => None,
                    }
                }

                /// Extract the application label from FCI if available
                pub fn application_label(&self) -> Option<bytes::Bytes> {
                    self.fci().and_then(|fci| crate::util::tlv::find_tlv_value(bytes::Bytes::copy_from_slice(fci), crate::constants::tags::APPLICATION_LABEL))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apdu_core::ApduCommand;
    use hex_literal::hex;

    #[test]
    fn test_select_command() {
        // Test SELECT command with AID
        let aid = hex!("A0000000030000");
        let cmd = SelectCommand::with_aid(aid.to_vec());

        assert_eq!(cmd.class(), cla::ISO7816);
        assert_eq!(cmd.instruction(), ins::SELECT);
        assert_eq!(cmd.p1(), select_p1::BY_NAME);
        assert_eq!(cmd.p2(), 0x00);
        assert_eq!(cmd.data(), Some(aid.as_ref()));
        assert_eq!(cmd.expected_length(), Some(0x00));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("00A4040007A000000003000000"));
    }

    #[test]
    fn test_select_response() {
        // Test successful response with FCI
        let fci_data = hex!("6F10840E315041592E5359532E4444463031A5020500");
        let mut response_data = Vec::new();
        response_data.extend_from_slice(&fci_data);
        response_data.extend_from_slice(&hex!("9000"));

        let response = SelectResponse::from_bytes(&response_data).unwrap();
        assert!(response.is_success());
        assert_eq!(response.fci(), Some(fci_data.as_ref()));

        // Test file not found
        let response_data = hex!("6A82");
        let response = SelectResponse::from_bytes(&response_data).unwrap();
        assert!(response.is_not_found());
        assert_eq!(response.fci(), None);
    }

    #[test]
    fn test_application_label_extraction() {
        // Create an FCI with an application label
        let fci_data = hex!("6F1A840E315041592E5359532E4444463031500841 50504C4142454C");
        //                                                 ^-- Application Label tag
        //                                                    ^-- "APPLABEL" in ASCII

        let mut response_data = Vec::new();
        response_data.extend_from_slice(&fci_data);
        response_data.extend_from_slice(&hex!("9000"));

        let response = SelectResponse::from_bytes(&response_data).unwrap();

        // Extract the application label
        let label = response.application_label();
        assert_eq!(
            label,
            Some(bytes::Bytes::from(hex!("4150504C4142454C").to_vec()))
        ); // "APPLABEL"
    }
}
