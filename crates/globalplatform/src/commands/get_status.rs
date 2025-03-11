//! GET STATUS command for GlobalPlatform
//!
//! This command is used to retrieve information about applications,
//! security domains, and load files on the card.

use apdu_macros::apdu_pair;

use crate::constants::{cla, get_status_p1, get_status_p2, ins, status, tags};

apdu_pair! {
    /// GET STATUS command for GlobalPlatform
    pub struct GetStatus {
        command {
            cla: cla::GP,
            ins: ins::GET_STATUS,
            secure: true,

            builders {
                /// Create a new GET STATUS command with specific P1 and AID filter
                pub fn with_aid_filter(p1: u8, aid: impl AsRef<[u8]>) -> Self {
                    // Build data field: tag + length + AID
                    let aid_data = aid.as_ref();
                    let mut data = Vec::with_capacity(2 + aid_data.len());
                    data.push(tags::GET_STATUS_AID);
                    data.push(aid_data.len() as u8);
                    data.extend_from_slice(aid_data);

                    Self::new(p1, get_status_p2::TLV_DATA).with_data(data)
                }

                /// Get status of issuer security domain
                pub fn issuer_security_domain(aid: impl AsRef<[u8]>) -> Self {
                    Self::with_aid_filter(get_status_p1::ISSUER_SECURITY_DOMAIN, aid)
                }

                /// Get status of applications
                pub fn applications(aid: impl AsRef<[u8]>) -> Self {
                    Self::with_aid_filter(get_status_p1::APPLICATIONS, aid)
                }

                /// Get status of executable load files
                pub fn executable_load_files(aid: impl AsRef<[u8]>) -> Self {
                    Self::with_aid_filter(get_status_p1::EXEC_LOAD_FILES, aid)
                }

                /// Get status of executable load files and modules
                pub fn executable_load_files_and_modules(aid: impl AsRef<[u8]>) -> Self {
                    Self::with_aid_filter(get_status_p1::EXEC_LOAD_FILES_AND_MODULES, aid)
                }

                /// Get status with empty AID (wildcard)
                pub fn all_with_type(p1: u8) -> Self {
                    Self::with_aid_filter(p1, &[])
                }
            }
        }

        response {
            variants {
                #[sw(status::SUCCESS)]
                Success {
                    tlv_data: Vec<u8>,
                },

                /// More data available (61xx)
                #[sw(0x61, _)]
                MoreData {
                    sw2: u8,
                    tlv_data: Vec<u8>,
                },

                /// Referenced data not found (6A88)
                #[sw(status::REFERENCED_DATA_NOT_FOUND)]
                ReferencedDataNotFound,

                /// Security condition not satisfied (6982)
                #[sw(status::SECURITY_CONDITION_NOT_SATISFIED)]
                SecurityConditionNotSatisfied,

                /// Other error
                #[sw(_, _)]
                OtherError {
                    sw1: u8,
                    sw2: u8,
                }
            }

            parse_payload = |payload: &[u8], _sw: apdu_core::StatusWord, variant: &mut Self| -> Result<(), apdu_core::Error> {
                match variant {
                    Self::Success { tlv_data } | Self::MoreData { tlv_data, .. } => {
                        tlv_data.extend_from_slice(payload);
                    }
                    _ => {}
                }
                Ok(())
            }

            methods {
                /// Get the TLV data
                pub fn tlv_data(&self) -> Option<&[u8]> {
                    match self {
                        Self::Success { tlv_data } | Self::MoreData { tlv_data, .. } => Some(tlv_data),
                        _ => None,
                    }
                }

                /// Check if more data is available
                pub fn has_more_data(&self) -> bool {
                    matches!(self, Self::MoreData { .. })
                }

                /// Get the number of remaining bytes if more data is available
                pub fn remaining_bytes(&self) -> Option<u8> {
                    match self {
                        Self::MoreData { sw2, .. } => Some(*sw2),
                        _ => None,
                    }
                }

                /// Parse application entries
                pub fn parse_applications(&self) -> Vec<ApplicationInfo> {
                    if let Some(data) = self.tlv_data() {
                        parse_application_entries(data)
                    } else {
                        Vec::new()
                    }
                }

                /// Parse load file entries
                pub fn parse_load_files(&self) -> Vec<LoadFileInfo> {
                    if let Some(data) = self.tlv_data() {
                        parse_load_file_entries(data)
                    } else {
                        Vec::new()
                    }
                }
            }
        }
    }
}

/// Application information from GET STATUS
#[derive(Debug, Clone)]
pub struct ApplicationInfo {
    /// AID of the application
    pub aid: Vec<u8>,
    /// Lifecycle state
    pub lifecycle: u8,
    /// Privileges
    pub privileges: Vec<u8>,
}

/// Load file information from GET STATUS
#[derive(Debug, Clone)]
pub struct LoadFileInfo {
    /// AID of the load file
    pub aid: Vec<u8>,
    /// Lifecycle state
    pub lifecycle: u8,
    /// Associated modules (if requested)
    pub modules: Vec<Vec<u8>>,
}

/// Parse application entries from GET STATUS response
fn parse_application_entries(data: &[u8]) -> Vec<ApplicationInfo> {
    let mut result = Vec::new();
    let mut index = 0;

    while index < data.len() {
        // Application entries are marked with E3 tag
        if data[index] == 0xE3 {
            let len = data[index + 1] as usize;
            let entry_end = (index + 2 + len).min(data.len());
            let entry_data = &data[index + 2..entry_end];

            // Parse the entry
            if let Some(info) = parse_application_entry(entry_data) {
                result.push(info);
            }
        }

        // Move to next TLV entry
        if index + 1 < data.len() {
            let len = data[index + 1] as usize;
            index += 2 + len;
        } else {
            break;
        }
    }

    result
}

/// Parse load file entries from GET STATUS response
fn parse_load_file_entries(data: &[u8]) -> Vec<LoadFileInfo> {
    let mut result = Vec::new();
    let mut index = 0;

    while index < data.len() {
        // Load file entries are marked with E2 tag
        if data[index] == 0xE2 {
            let len = data[index + 1] as usize;
            let entry_end = (index + 2 + len).min(data.len());
            let entry_data = &data[index + 2..entry_end];

            // Parse the entry
            if let Some(info) = parse_load_file_entry(entry_data) {
                result.push(info);
            }
        }

        // Move to next TLV entry
        if index + 1 < data.len() {
            let len = data[index + 1] as usize;
            index += 2 + len;
        } else {
            break;
        }
    }

    result
}

/// Parse a single application entry
fn parse_application_entry(data: &[u8]) -> Option<ApplicationInfo> {
    // Find AID (4F tag)
    let aid = crate::util::tlv::find_tlv_value(data, 0x4F)?;

    // Find lifecycle (C5 tag, optional)
    let lifecycle = crate::util::tlv::find_tlv_value(data, 0xC5)
        .map_or(0, |d| if d.is_empty() { 0 } else { d[0] });

    // Find privileges (C6 tag, optional)
    let privileges =
        crate::util::tlv::find_tlv_value(data, 0xC6).map_or(Vec::new(), |d| d.to_vec());

    Some(ApplicationInfo {
        aid: aid.to_vec(),
        lifecycle,
        privileges,
    })
}

/// Parse a single load file entry
fn parse_load_file_entry(data: &[u8]) -> Option<LoadFileInfo> {
    // Find AID (4F tag)
    let aid = crate::util::tlv::find_tlv_value(data, 0x4F)?;

    // Find lifecycle (C5 tag, optional)
    let lifecycle = crate::util::tlv::find_tlv_value(data, 0xC5)
        .map_or(0, |d| if d.is_empty() { 0 } else { d[0] });

    // Find modules (84 tag, may be multiple)
    let modules = crate::util::tlv::find_all_tlv_values(data, 0x84)
        .into_iter()
        .map(|d| d.to_vec())
        .collect();

    Some(LoadFileInfo {
        aid: aid.to_vec(),
        lifecycle,
        modules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use apdu_core::ApduCommand;
    use hex_literal::hex;

    #[test]
    fn test_get_status_command() {
        let aid = hex!("AABBCC");
        let cmd = GetStatusCommand::applications(&aid);

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::GET_STATUS);
        assert_eq!(cmd.p1(), get_status_p1::APPLICATIONS);
        assert_eq!(cmd.p2(), get_status_p2::TLV_DATA);

        // Check data format (tag + length + AID)
        let expected_data = hex!("4F03AABBCC");
        assert_eq!(cmd.data(), Some(expected_data.as_ref()));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(raw.as_ref(), hex!("80F240020A4F03AABBCC"));
    }

    #[test]
    fn test_get_status_all() {
        let cmd = GetStatusCommand::all_with_type(get_status_p1::APPLICATIONS);

        // Check data format (tag + length + empty AID)
        let expected_data = hex!("4F00");
        assert_eq!(cmd.data(), Some(expected_data.as_ref()));
    }

    #[test]
    fn test_get_status_response() {
        // Test successful response with TLV data
        let tlv_data = hex!("E3144F07A0000000030000C5010AC4019AC10100860102");
        let response_data = [tlv_data.as_ref(), &hex!("9000")].concat();

        let response = GetStatusResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, GetStatusResponse::Success { .. }));
        assert_eq!(response.tlv_data(), Some(tlv_data.as_ref()));
        assert!(!response.has_more_data());

        // Test more data available
        let tlv_data = hex!("E3144F07A0000000030000C5010AC4019AC10100860102");
        let response_data = [tlv_data.as_ref(), &hex!("6120")].concat();

        let response = GetStatusResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, GetStatusResponse::MoreData { .. }));
        assert_eq!(response.tlv_data(), Some(tlv_data.as_ref()));
        assert!(response.has_more_data());
        assert_eq!(response.remaining_bytes(), Some(0x20));
    }

    #[test]
    fn test_parse_application_entries() {
        // Create sample data with two application entries
        let tlv_data = hex!(
            "E3134F07A0000000030000C5010AC60106" // First app
            "E3164F09A000000003000001C50104C60301FF02" // Second app
        );

        let response_data = [tlv_data.as_ref(), &hex!("9000")].concat();
        let response = GetStatusResponse::from_bytes(&response_data).unwrap();

        // Parse applications
        let apps = response.parse_applications();

        // Check that we got two applications
        assert_eq!(apps.len(), 2);

        // Check first app
        assert_eq!(apps[0].aid, hex!("A0000000030000"));
        assert_eq!(apps[0].lifecycle, 0x0A);
        assert_eq!(apps[0].privileges, hex!("06"));

        // Check second app
        assert_eq!(apps[1].aid, hex!("A000000003000001"));
        assert_eq!(apps[1].lifecycle, 0x04);
        assert_eq!(apps[1].privileges, hex!("01FF02"));
    }

    #[test]
    fn test_parse_load_file_entries() {
        // Create sample data with two load file entries
        let tlv_data = hex!(
            "E2114F07A0000000030000C50107" // First load file, no modules
            "E21C4F09A000000003000102C50108840AA000000003000102A1" // Second load file with module
        );

        let response_data = [tlv_data.as_ref(), &hex!("9000")].concat();
        let response = GetStatusResponse::from_bytes(&response_data).unwrap();

        // Parse load files
        let files = response.parse_load_files();

        // Check that we got two load files
        assert_eq!(files.len(), 2);

        // Check first load file
        assert_eq!(files[0].aid, hex!("A0000000030000"));
        assert_eq!(files[0].lifecycle, 0x07);
        assert_eq!(files[0].modules.len(), 0);

        // Check second load file
        assert_eq!(files[1].aid, hex!("A000000003000102"));
        assert_eq!(files[1].lifecycle, 0x08);
        assert_eq!(files[1].modules.len(), 1);
        assert_eq!(files[1].modules[0], hex!("A000000003000102A1"));
    }
}
