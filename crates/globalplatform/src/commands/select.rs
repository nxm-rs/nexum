//! SELECT command for GlobalPlatform
//!
//! This command is used to select an application or file by its AID.

use bytes::Bytes;
use iso7816_tlv::ber::{Tlv, Value};
use nexum_apdu_macros::apdu_pair;
use std::convert::TryFrom;

use crate::constants::{cla, ins, select_p1, status};

/// Represents the parsed FCI (File Control Information)
#[derive(Debug, Clone)]
pub struct FciTemplate {
    /// Application/file AID (tag 84)
    pub aid: Bytes,
    /// Proprietary data (tag A5)
    pub proprietary_data: ProprietaryData,
}

/// Represents proprietary data within the FCI
#[derive(Debug, Clone)]
pub struct ProprietaryData {
    /// Security Domain Management Data (tag 73)
    pub security_domain_management_data: Option<Bytes>,
    /// Application production life cycle data (tag 9F6E)
    pub app_production_lifecycle_data: Option<Bytes>,
    /// Maximum length of data field in command message (tag 9F65)
    pub max_command_data_length: u16,
}

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
                #[payload(field = "fci")]
                Success {
                    fci: Vec<u8>,
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

            methods {
                /// Returns true if the selection was successful
                pub const fn is_success(&self) -> bool {
                    matches!(self, Self::Success { .. })
                }

                /// Returns true if the file or application was not found
                pub const fn is_not_found(&self) -> bool {
                    matches!(self, Self::NotFound { .. })
                }

                /// Get the File Control Information if available
                pub fn fci(&self) -> &[u8] {
                    match self {
                        Self::Success { fci } => fci.as_slice(),
                        _ => &[],
                    }
                }

                /// Extract the application label from FCI if available
                pub fn application_label(&self) -> Option<bytes::Bytes> {
                    crate::util::tlv::find_tlv_value(bytes::Bytes::copy_from_slice(self.fci()), crate::constants::tags::APPLICATION_LABEL)
                }

                /// Parse the FCI data into a structured format
                pub fn parsed_fci(&self) -> Option<FciTemplate> {
                    parse_fci(self.fci()).ok()
                }
            }
        }
    }
}

// FCI tag constants
const TAG_FCI_TEMPLATE: u8 = 0x6F;
const TAG_AID: u8 = 0x84;
const TAG_PROPRIETARY_DATA: u8 = 0xA5;
const TAG_SECURITY_DOMAIN_MGMT_DATA: u8 = 0x73;
const TAG_APP_PRODUCTION_LIFECYCLE_DATA: u16 = 0x9F6E;
const TAG_MAX_COMMAND_DATA_LENGTH: u16 = 0x9F65;

/// Parse the FCI data into a structured format
fn parse_fci(fci: &[u8]) -> Result<FciTemplate, &'static str> {
    // Parse the FCI template (tag 6F)
    let tlvs = Tlv::parse_all(fci);
    let fci_tlv = tlvs
        .iter()
        .find(|tlv| *tlv.tag() == u16::from(TAG_FCI_TEMPLATE).try_into().unwrap())
        .ok_or("FCI template (6F) not found")?;

    // Extract content of the FCI template
    if let Value::Constructed(content_tlvs) = fci_tlv.value() {
        // Find the AID (tag 84)
        let aid_tlv = content_tlvs
            .iter()
            .find(|tlv| *tlv.tag() == u16::from(TAG_AID).try_into().unwrap())
            .ok_or("AID (84) not found in FCI")?;

        // Extract AID value
        let aid = if let Value::Primitive(aid_data) = aid_tlv.value() {
            Bytes::copy_from_slice(aid_data)
        } else {
            return Err("Invalid AID value format (not primitive)");
        };

        // Find the proprietary data (tag A5)
        let prop_tlv = content_tlvs
            .iter()
            .find(|tlv| *tlv.tag() == u16::from(TAG_PROPRIETARY_DATA).try_into().unwrap())
            .ok_or("Proprietary data (A5) not found in FCI")?;

        // Parse proprietary data
        let proprietary_data = parse_proprietary_data(prop_tlv)?;

        Ok(FciTemplate {
            aid,
            proprietary_data,
        })
    } else {
        Err("FCI template (6F) is not constructed")
    }
}

/// Parse the proprietary data from the FCI
fn parse_proprietary_data(prop_tlv: &Tlv) -> Result<ProprietaryData, &'static str> {
    if let Value::Constructed(prop_content) = prop_tlv.value() {
        // Extract Security Domain Management Data (tag 73) if present
        let security_domain_management_data = prop_content
            .iter()
            .find(|tlv| *tlv.tag() == u16::from(TAG_SECURITY_DOMAIN_MGMT_DATA).try_into().unwrap())
            .and_then(|tlv| {
                if let Value::Primitive(data) = tlv.value() {
                    Some(Bytes::copy_from_slice(data))
                } else {
                    None
                }
            });

        // Extract Application Production Life Cycle Data (tag 9F6E) if present
        let app_production_lifecycle_data = prop_content
            .iter()
            .find(|tlv| *tlv.tag() == TAG_APP_PRODUCTION_LIFECYCLE_DATA.try_into().unwrap())
            .and_then(|tlv| {
                if let Value::Primitive(data) = tlv.value() {
                    Some(Bytes::copy_from_slice(data))
                } else {
                    None
                }
            });

        // Extract Maximum Command Data Length (tag 9F65) - mandatory
        let max_command_data_length = prop_content
            .iter()
            .find(|tlv| *tlv.tag() == TAG_MAX_COMMAND_DATA_LENGTH.try_into().unwrap())
            .ok_or("Max command data length (9F65) not found")?;

        // Parse max command data length as u16
        if let Value::Primitive(max_length_value) = max_command_data_length.value() {
            let max_length = if max_length_value.len() >= 2 {
                ((max_length_value[0] as u16) << 8) | (max_length_value[1] as u16)
            } else if max_length_value.len() == 1 {
                max_length_value[0] as u16
            } else {
                return Err("Invalid max command data length format");
            };

            Ok(ProprietaryData {
                security_domain_management_data,
                app_production_lifecycle_data,
                max_command_data_length: max_length,
            })
        } else {
            Err("Invalid max command data length format (not primitive)")
        }
    } else {
        Err("Proprietary data (A5) is not constructed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use nexum_apdu_core::ApduCommand;

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
        assert_eq!(response.fci(), fci_data.as_slice());

        // Test file not found
        let response_data = hex!("6A82");
        let response = SelectResponse::from_bytes(&response_data).unwrap();
        assert!(response.is_not_found());
    }
}
