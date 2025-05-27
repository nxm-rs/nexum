//! GET STATUS command for GlobalPlatform
//!
//! This command is used to retrieve information about applications,
//! security domains, and load files on the card.

use bytes::Bytes;
use nexum_apdu_macros::apdu_pair;

use crate::constants::*;
use iso7816_tlv::simple::Tlv;

apdu_pair! {
    /// GET STATUS command for GlobalPlatform
    pub struct GetStatus {
        command {
            cla: cla::GP,
            ins: ins::GET_STATUS,
            required_security_level: SecurityLevel::mac_protected(),

            builders {
                /// Create a new GET STATUS command with specific P1 and AID filter
                pub fn with_aid_filter(p1: u8, aid: impl AsRef<[u8]>) -> Self {
                    // Build data field: tag + length + AID
                    let data = Tlv::new(tags::GET_STATUS_AID.try_into().unwrap(), aid.as_ref().to_vec()).unwrap();

                    Self::new(p1, get_status_p2::TLV_DATA).with_data(data.to_vec())
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
                    Self::with_aid_filter(p1, [])
                }
            }
        }

        response {
            ok {
                /// Successful response
                #[sw(SW_NO_ERROR)]
                #[payload(field = "tlv_data")]
                Success {
                    /// TLV data
                    tlv_data: Vec<u8>,
                },

                /// More data available (61xx)
                #[sw(0x61, _)]
                #[payload(field = "tlv_data")]
                MoreData {
                    /// Remaining bytes
                    sw2: u8,
                    /// Response data
                    tlv_data: Vec<u8>,
                },
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
            }


        }
    }
}

impl GetStatusOk {
    /// Get the TLV data
    pub const fn tlv_data(&self) -> &Vec<u8> {
        match self {
            Self::Success { tlv_data } | Self::MoreData { tlv_data, .. } => tlv_data,
        }
    }

    /// Check if more data is available
    pub const fn has_more_data(&self) -> bool {
        matches!(self, Self::MoreData { .. })
    }

    /// Get the number of remaining bytes if more data is available
    pub const fn remaining_bytes(&self) -> Option<u8> {
        match self {
            Self::MoreData { sw2, .. } => Some(*sw2),
            _ => None,
        }
    }
}

/// Parse application entries from GET STATUS response
///
/// This function extracts and parses application information from the TLV data
/// returned by the GET STATUS command. It converts the raw TLV data into a
/// structured format that's easier to work with.
///
/// # Parameters
/// * `data` - The successful response from the GET STATUS command
///
/// # Returns
/// A vector of parsed `ApplicationInfo` structs containing information about
/// each application found in the response data.
pub fn parse_applications(data: GetStatusOk) -> Vec<ApplicationInfo> {
    parse_entries(data.tlv_data().as_slice(), EntryType::Application)
        .into_iter()
        .filter_map(|entry| {
            if let Entry::Application(app) = entry {
                Some(app)
            } else {
                None
            }
        })
        .collect()
}

/// Parse load file entries from GET STATUS response
///
/// This function extracts and parses load file information from the TLV data
/// returned by the GET STATUS command. It converts the raw TLV data into a
/// structured format that's easier to work with.
///
/// # Parameters
/// * `data` - The successful response from the GET STATUS command
///
/// # Returns
/// A vector of parsed `LoadFileInfo` structs containing information about
/// each load file (package) found in the response data.
pub fn parse_load_files(data: GetStatusOk) -> Vec<LoadFileInfo> {
    parse_entries(data.tlv_data().as_slice(), EntryType::LoadFile)
        .into_iter()
        .filter_map(|entry| {
            if let Entry::LoadFile(file) = entry {
                Some(file)
            } else {
                None
            }
        })
        .collect()
}

/// Application information from GET STATUS
///
/// This struct represents information about an application or security domain
/// installed on the card, as returned by the GET STATUS command.
#[derive(Debug, Clone)]
pub struct ApplicationInfo {
    /// AID (Application Identifier) of the application
    pub aid: Bytes,
    /// Lifecycle state of the application (as defined in GlobalPlatform specifications)
    pub lifecycle: u8,
    /// Privileges assigned to the application
    pub privileges: Bytes,
}

/// Load file information from GET STATUS
///
/// This struct represents information about a load file (package)
/// installed on the card, as returned by the GET STATUS command.
#[derive(Debug, Clone)]
pub struct LoadFileInfo {
    /// AID (Application Identifier) of the load file
    pub aid: Bytes,
    /// Lifecycle state of the load file (as defined in GlobalPlatform specifications)
    pub lifecycle: u8,
    /// Associated executable modules (applets) contained in this load file
    pub modules: Vec<Bytes>,
}

/// Tag constants for GET STATUS response parsing
const TAG_AID: u8 = 0x4F;
const TAG_LIFECYCLE: u8 = 0xC5;
const TAG_PRIVILEGES: u8 = 0xC6;
const TAG_MODULE_AID: u8 = 0x84;
const TAG_APPLICATION: u8 = 0xE3;
const TAG_LOAD_FILE: u8 = 0xE2;

/// Type of entry to parse in GET STATUS response
///
/// Represents the different types of entries that can be found in a GET STATUS response,
/// which have different tag values and structure.
#[derive(Debug, Copy, Clone)]
enum EntryType {
    /// Application or security domain entry (tag E3)
    Application,
    /// Executable load file (package) entry (tag E2)
    LoadFile,
}

impl EntryType {
    /// Returns the TLV tag value for this entry type
    const fn tag(&self) -> u8 {
        match self {
            Self::Application => TAG_APPLICATION,
            Self::LoadFile => TAG_LOAD_FILE,
        }
    }
}

/// Parsed entry from GET STATUS response
///
/// Represents an entry parsed from the GET STATUS response TLV data,
/// which can be either an application or a load file.
enum Entry {
    /// Application or security domain entry
    Application(ApplicationInfo),
    /// Executable load file (package) entry
    LoadFile(LoadFileInfo),
}

/// Parse all entries of a specific type from the response data
///
/// This function extracts all entries of the specified type from the raw TLV data
/// returned by the GET STATUS command.
///
/// # Parameters
/// * `data` - The raw TLV data from the GET STATUS response
/// * `entry_type` - The type of entries to extract (Application or LoadFile)
///
/// # Returns
/// A vector of parsed entries matching the specified type
fn parse_entries(data: &[u8], entry_type: EntryType) -> Vec<Entry> {
    // Parse all TLVs at the top level
    let tlvs = Tlv::parse_all(data);

    // Filter for TLVs with the matching tag and parse them
    tlvs.iter()
        .filter(|tlv| Into::<u8>::into(tlv.tag()) == entry_type.tag())
        .filter_map(|tlv| parse_entry(tlv.value(), entry_type))
        .collect()
}

/// Parse a single entry (application or load file) from TLV data
///
/// This function parses the contents of a single TLV with tag E2 or E3
/// to extract the fields that make up an application or load file entry.
///
/// # Parameters
/// * `data` - The value part of a top-level TLV representing an entry
/// * `entry_type` - The type of entry to parse (Application or LoadFile)
///
/// # Returns
/// An optional parsed entry if successful, or None if required fields are missing
fn parse_entry(data: &[u8], entry_type: EntryType) -> Option<Entry> {
    // Parse inner TLVs
    let tlvs = Tlv::parse_all(data);

    // Extract common fields
    let mut aid = None;
    let mut lifecycle = 0;
    let mut privileges = Bytes::new();
    let mut modules = Vec::new();

    // Extract data from TLVs
    for tlv in &tlvs {
        match Into::<u8>::into(tlv.tag()) {
            TAG_AID => aid = Some(Bytes::copy_from_slice(tlv.value())),
            TAG_LIFECYCLE => {
                if !tlv.value().is_empty() {
                    lifecycle = tlv.value()[0];
                }
            }
            TAG_PRIVILEGES => privileges = Bytes::copy_from_slice(tlv.value()),
            TAG_MODULE_AID => {
                if matches!(entry_type, EntryType::LoadFile) {
                    modules.push(Bytes::copy_from_slice(tlv.value()));
                }
            }
            _ => {} // Ignore other tags
        }
    }

    // AID is required
    aid.map(|aid_value| match entry_type {
        EntryType::Application => Entry::Application(ApplicationInfo {
            aid: aid_value,
            lifecycle,
            privileges,
        }),
        EntryType::LoadFile => Entry::LoadFile(LoadFileInfo {
            aid: aid_value,
            lifecycle,
            modules,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use hex_literal::hex;
    use nexum_apdu_core::ApduCommand;

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
        assert_eq!(raw.as_ref(), hex!("80F2400205" "4F03AABBCC"));
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
        let mut buf = BytesMut::new();
        buf.put(tlv_data.as_ref());
        buf.put(hex!("9000").as_ref());

        let result = GetStatusCommand::parse_response_raw(buf.freeze()).unwrap();
        assert!(matches!(result, GetStatusOk::Success { .. }));
        assert_eq!(result.tlv_data(), &tlv_data.to_vec());
        assert!(!result.has_more_data());

        // Test more data available
        let tlv_data = hex!("E3144F07A0000000030000C5010AC4019AC10100860102");
        let mut buf = BytesMut::new();
        buf.put(tlv_data.as_ref());
        buf.put(hex!("6120").as_ref());

        let result = GetStatusCommand::parse_response_raw(buf.freeze()).unwrap();
        assert!(matches!(result, GetStatusOk::MoreData { .. }));
        assert_eq!(result.tlv_data(), &tlv_data.to_vec());
        assert!(result.has_more_data());
        assert_eq!(result.remaining_bytes(), Some(0x20));
    }

    #[test]
    fn test_parse_application_entries() {
        // Create response data
        let response_data = Bytes::from_static(&hex!(
            "E30F4F07A0000000030000C5010AC60106"
            "E3124F08A000000003000001C50104C60301FF02"
            "9000"
        ));

        let result = GetStatusCommand::parse_response_raw(response_data).unwrap();

        // Parse applications
        let apps = parse_applications(result);

        // Check that we got two applications
        assert_eq!(apps.len(), 2);

        // Check first app
        assert_eq!(apps[0].aid, Bytes::copy_from_slice(&hex!("A0000000030000")));
        assert_eq!(apps[0].lifecycle, 0x0A);
        assert_eq!(apps[0].privileges, Bytes::copy_from_slice(&hex!("06")));

        // Check second app
        assert_eq!(
            apps[1].aid,
            Bytes::copy_from_slice(&hex!("A000000003000001"))
        );
        assert_eq!(apps[1].lifecycle, 0x04);
        assert_eq!(apps[1].privileges, Bytes::copy_from_slice(&hex!("01FF02")));
    }

    #[test]
    fn test_parse_load_file_entries() {
        // Create response data
        let response_data = Bytes::from_static(&hex!(
            "E20C4F07A0000000030000C50107"
            "E2184F08A000000003000102C501088409A000000003000102A1"
            "9000"
        ));

        let result = GetStatusCommand::parse_response_raw(response_data).unwrap();

        // Parse load files
        let files = parse_load_files(result);

        // Check that we got two load files
        assert_eq!(files.len(), 2);

        // Check first load file
        assert_eq!(
            files[0].aid,
            Bytes::copy_from_slice(&hex!("A0000000030000"))
        );
        assert_eq!(files[0].lifecycle, 0x07);
        assert_eq!(files[0].modules.len(), 0);

        // Check second load file
        assert_eq!(
            files[1].aid,
            Bytes::copy_from_slice(&hex!("A000000003000102"))
        );
        assert_eq!(files[1].lifecycle, 0x08);
        assert_eq!(files[1].modules.len(), 1);
        assert_eq!(
            files[1].modules[0],
            Bytes::copy_from_slice(&hex!("A000000003000102A1"))
        );
    }
}
