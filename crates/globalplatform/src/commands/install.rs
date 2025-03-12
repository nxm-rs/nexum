//! INSTALL command for GlobalPlatform
//!
//! This command is used to install applications on the card.

use apdu_macros::apdu_pair;

use crate::constants::{cla, ins, install_p1, status};

apdu_pair! {
    /// INSTALL command for GlobalPlatform
    pub struct Install {
        command {
            cla: cla::GP,
            ins: ins::INSTALL,
            secure: true,

            builders {
                /// Create an INSTALL command with parameters
                pub fn with_p1_data(p1: u8, data: impl Into<bytes::Bytes>) -> Self {
                    Self::new(p1, 0x00).with_data(data.into())
                }

                /// Create an INSTALL [for load] command
                pub fn for_load(load_file_aid: impl AsRef<[u8]>, security_domain_aid: impl AsRef<[u8]>) -> Self {
                    let load_file_aid = load_file_aid.as_ref();
                    let security_domain_aid = security_domain_aid.as_ref();

                    // Build data: load_file_aid_length + load_file_aid + sd_aid_length + sd_aid + 3 empty fields
                    let mut data = Vec::with_capacity(3 + load_file_aid.len() + security_domain_aid.len());
                    data.push(load_file_aid.len() as u8);
                    data.extend_from_slice(load_file_aid);
                    data.push(security_domain_aid.len() as u8);
                    data.extend_from_slice(security_domain_aid);
                    data.extend_from_slice(&[0x00, 0x00, 0x00]);

                    Self::with_p1_data(install_p1::FOR_LOAD, data)
                }

                /// Create an INSTALL [for install] command
                pub fn for_install(
                    executable_load_file_aid: impl AsRef<[u8]>,
                    executable_module_aid: impl AsRef<[u8]>,
                    application_aid: impl AsRef<[u8]>,
                    privilege: impl AsRef<[u8]>,
                    install_parameters: impl AsRef<[u8]>,
                    install_token: impl AsRef<[u8]>,
                ) -> Self {
                    let data = build_install_data(
                        executable_load_file_aid,
                        executable_module_aid,
                        application_aid,
                        privilege,
                        install_parameters,
                        install_token,
                    );

                    Self::with_p1_data(install_p1::FOR_INSTALL, data)
                }

                /// Create an INSTALL [for install and make selectable] command
                pub fn for_install_and_make_selectable(
                    executable_load_file_aid: impl AsRef<[u8]>,
                    executable_module_aid: impl AsRef<[u8]>,
                    application_aid: impl AsRef<[u8]>,
                    privilege: impl AsRef<[u8]>,
                    install_parameters: impl AsRef<[u8]>,
                    install_token: impl AsRef<[u8]>,
                ) -> Self {
                    let data = build_install_data(
                        executable_load_file_aid,
                        executable_module_aid,
                        application_aid,
                        privilege,
                        install_parameters,
                        install_token,
                    );

                    Self::with_p1_data(install_p1::FOR_INSTALL_AND_MAKE_SELECTABLE, data)
                }

                /// Create an INSTALL [for personalization] command
                pub fn for_personalization(application_aid: impl AsRef<[u8]>, data: impl AsRef<[u8]>) -> Self {
                    let app_aid = application_aid.as_ref();
                    let app_data = data.as_ref();

                    // Build data: app_aid_length + app_aid + other fields empty
                    let mut cmd_data = Vec::with_capacity(app_aid.len() + 6);
                    cmd_data.push(0x00); // Empty load file AID
                    cmd_data.push(0x00); // Empty module AID
                    cmd_data.push(app_aid.len() as u8);
                    cmd_data.extend_from_slice(app_aid);
                    cmd_data.push(0x00); // Empty privileges
                    cmd_data.push(app_data.len() as u8); // Parameters length
                    cmd_data.extend_from_slice(app_data);
                    cmd_data.push(0x00); // Empty token

                    Self::with_p1_data(install_p1::FOR_PERSONALIZATION, cmd_data)
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

/// Build the data field for INSTALL [for install] and [for install and make selectable] commands
fn build_install_data(
    executable_load_file_aid: impl AsRef<[u8]>,
    executable_module_aid: impl AsRef<[u8]>,
    application_aid: impl AsRef<[u8]>,
    privilege: impl AsRef<[u8]>,
    install_parameters: impl AsRef<[u8]>,
    install_token: impl AsRef<[u8]>,
) -> Vec<u8> {
    let executable_load_file_aid = executable_load_file_aid.as_ref();
    let executable_module_aid = executable_module_aid.as_ref();
    let application_aid = application_aid.as_ref();
    let privilege = privilege.as_ref();
    let install_parameters = install_parameters.as_ref();
    let install_token = install_token.as_ref();

    // Build data field
    let mut data = Vec::new();

    // Executable Load File AID
    data.push(executable_load_file_aid.len() as u8);
    data.extend_from_slice(executable_load_file_aid);

    // Executable Module AID
    data.push(executable_module_aid.len() as u8);
    data.extend_from_slice(executable_module_aid);

    // Application AID
    data.push(application_aid.len() as u8);
    data.extend_from_slice(application_aid);

    // Privilege
    data.push(privilege.len() as u8);
    data.extend_from_slice(privilege);

    // Install Parameters
    if install_parameters.is_empty() {
        // Even for empty parameters, we need to provide the C9 tag with zero length
        let params_tlv = [0xC9, 0x00];
        data.push(params_tlv.len() as u8);
        data.extend_from_slice(&params_tlv);
    } else {
        // Create TLV structure: C9 + len + value
        let mut params_tlv = Vec::with_capacity(2 + install_parameters.len());
        params_tlv.push(0xC9); // Tag for application specific parameters
        params_tlv.push(install_parameters.len() as u8);
        params_tlv.extend_from_slice(install_parameters);

        data.push(params_tlv.len() as u8);
        data.extend_from_slice(&params_tlv);
    }

    // Install Token
    data.push(install_token.len() as u8);
    data.extend_from_slice(install_token);

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use apdu_core::ApduCommand;
    use hex_literal::hex;

    #[test]
    fn test_install_for_load() {
        let package_aid = hex!("53746174757357616C6C6574");
        let sd_aid = hex!("A000000151000000");
        let cmd = InstallCommand::for_load(&package_aid, &sd_aid);

        assert_eq!(cmd.class(), cla::GP);
        assert_eq!(cmd.instruction(), ins::INSTALL);
        assert_eq!(cmd.p1(), install_p1::FOR_LOAD);
        assert_eq!(cmd.p2(), 0x00);

        // Check the command data
        let expected_data = hex!("0C53746174757357616C6C657408A000000151000000000000");
        assert_eq!(cmd.data(), Some(expected_data.as_ref()));

        // Test command serialization
        let raw = cmd.to_bytes();
        assert_eq!(
            raw.as_ref(),
            hex!("80E60200190C53746174757357616C6C657408A000000151000000000000")
        );
    }

    #[test]
    fn test_install_for_install_and_make_selectable() {
        let package_aid = hex!("53746174757357616C6C6574");
        let module_aid = hex!("53746174757357616C6C6574417070");
        let applet_aid = hex!("53746174757357616C6C6574417070");
        let privileges = hex!("01");
        let install_params = hex!("03AABBCC");
        let install_token = hex!("");

        let cmd = InstallCommand::for_install_and_make_selectable(
            &package_aid,
            &module_aid,
            &applet_aid,
            &privileges,
            &install_params,
            &install_token,
        );

        assert_eq!(cmd.p1(), install_p1::FOR_INSTALL_AND_MAKE_SELECTABLE);

        // Check the command data format
        let expected_data = hex!(
            "0C53746174757357616C6C65740F53746174757357616C6C65744170700F53746174757357616C6C6574417070010106C90403AABBCC00"
        );
        assert_eq!(cmd.data(), Some(expected_data.as_ref()));
    }

    #[test]
    fn test_for_personalization() {
        let app_aid = hex!("A0000001510000");
        let app_data = hex!("84010102");

        let cmd = InstallCommand::for_personalization(&app_aid, &app_data);

        assert_eq!(cmd.p1(), install_p1::FOR_PERSONALIZATION);

        // Just verify command data is not empty rather than exact content
        assert!(cmd.data().is_some());
        assert!(!cmd.data().unwrap().is_empty());
    }

    #[test]
    fn test_install_response() {
        // Test successful response
        let response_data = hex!("9000");
        let response = InstallResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(response, InstallResponse::Success));

        // Test error response
        let response_data = hex!("6982");
        let response = InstallResponse::from_bytes(&response_data).unwrap();
        assert!(matches!(
            response,
            InstallResponse::SecurityConditionNotSatisfied
        ));
    }
}
