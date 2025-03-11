//! GlobalPlatform application implementation
//!
//! This module provides the main GlobalPlatform application interface,
//! which encapsulates all the functionality for managing smart cards.

use apdu_core::ApduCommand;
use rand::RngCore;

use apdu_core::prelude::{Executor, ResponseAwareExecutor, SecureChannelExecutor};
use apdu_core::{Bytes, Command, StatusWord, processor::secure::SecurityLevel};

use crate::{
    Error, Result,
    commands::{
        DeleteCommand, DeleteResponse, ExternalAuthenticateCommand, ExternalAuthenticateResponse,
        GetStatusCommand, GetStatusResponse, InstallCommand, InstallResponse, LoadCommand,
        LoadResponse, SelectCommand, SelectResponse,
        initialize_update::{InitializeUpdateCommand, InitializeUpdateResponse},
    },
    constants::{
        DEFAULT_HOST_CHALLENGE_LENGTH, SECURITY_DOMAIN_AID, external_auth_p1, get_status_p1,
        load_p1,
    },
    load::{CapFileInfo, LoadCommandStream},
    secure_channel::create_secure_channel_provider,
    session::{Keys, Session},
};

/// Default GlobalPlatform keys
pub struct DefaultKeys;

impl DefaultKeys {
    /// Create a new set of default GlobalPlatform keys
    pub fn new() -> Keys {
        // Default GlobalPlatform test key
        let key = [
            0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D,
            0x4E, 0x4F,
        ];
        Keys::from_single_key(key)
    }
}

/// GlobalPlatform card management application
pub struct GlobalPlatform<E: Executor + ResponseAwareExecutor + SecureChannelExecutor> {
    /// Card executor
    executor: E,
    /// Current session
    session: Option<Session>,
    /// Last response for session creation
    last_response: Option<Bytes>,
}

impl<E: Executor + ResponseAwareExecutor + SecureChannelExecutor> GlobalPlatform<E> {
    /// Create a new GlobalPlatform instance
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            session: None,
            last_response: None,
        }
    }

    /// Select the card manager (ISD)
    pub fn select_card_manager(&mut self) -> Result<SelectResponse> {
        self.select_application(SECURITY_DOMAIN_AID)
    }

    /// Select an application by AID
    pub fn select_application(&mut self, aid: &[u8]) -> Result<SelectResponse> {
        // Create SELECT command
        let cmd = SelectCommand::with_aid(aid.to_vec());

        // Execute command
        match self.executor.execute(&cmd) {
            Ok(response) => {
                // Store response for possible later use
                if let Ok(raw_response) = self.executor.last_response() {
                    self.last_response = Some(Bytes::copy_from_slice(raw_response));
                }
                Ok(response)
            }
            Err(e) => Err(Error::from(e)),
        }
    }

    /// Open a secure channel with default keys
    pub fn open_secure_channel(&mut self) -> Result<()> {
        self.open_secure_channel_with_keys(&DefaultKeys::new(), SecurityLevel::MACProtection)
    }

    /// Open a secure channel with specific keys and security level
    pub fn open_secure_channel_with_keys(
        &mut self,
        keys: &Keys,
        level: SecurityLevel,
    ) -> Result<()> {
        // Generate a random host challenge
        let mut host_challenge = [0u8; DEFAULT_HOST_CHALLENGE_LENGTH];
        rand::rng().fill_bytes(&mut host_challenge);

        // Initialize update
        let init_cmd = InitializeUpdateCommand::with_challenge(host_challenge.to_vec());
        let init_response = match self.executor.execute(&init_cmd) {
            Ok(response) => {
                // Store raw response bytes for session creation
                if let Ok(raw_response) = self.executor.last_response() {
                    self.last_response = Some(Bytes::copy_from_slice(raw_response));
                }
                response
            }
            Err(e) => return Err(Error::from(e)),
        };

        // Check the response
        if !matches!(init_response, InitializeUpdateResponse::Success { .. }) {
            return Err(Error::AuthenticationFailed("INITIALIZE UPDATE failed"));
        }

        // Create a new session from the response
        let response_bytes = self
            .last_response
            .as_ref()
            .ok_or(Error::AuthenticationFailed("Missing response data"))?;

        let session = match &init_response {
            InitializeUpdateResponse::Success { .. } => {
                Session::new(keys, response_bytes, &host_challenge)?
            }
            _ => return Err(Error::AuthenticationFailed("INITIALIZE UPDATE failed")),
        };

        // Store the session
        self.session = Some(session.clone());

        // Determine the security level parameter
        let security_level_p1 = match level {
            SecurityLevel::MACProtection => external_auth_p1::CMAC,
            SecurityLevel::FullEncryption => external_auth_p1::CMAC | external_auth_p1::ENC,
            _ => external_auth_p1::CMAC, // Default to MAC protection
        };

        // Create external authenticate command with appropriate security level
        let auth_cmd = ExternalAuthenticateCommand::with_security_level(
            session.keys().enc(),
            session.card_challenge(),
            session.host_challenge(),
            security_level_p1,
        )?;

        // Create secure channel provider
        let provider = create_secure_channel_provider(session);

        // Open the secure channel
        self.executor.open_secure_channel(&provider, level)?;

        // Send external authenticate command through the secure channel
        let auth_response = self.executor.execute(&auth_cmd)?;

        // Check if authentication was successful
        if !matches!(auth_response, ExternalAuthenticateResponse::Success) {
            return Err(Error::AuthenticationFailed("EXTERNAL AUTHENTICATE failed"));
        }

        Ok(())
    }

    /// Delete an object
    pub fn delete_object(&mut self, aid: &[u8]) -> Result<DeleteResponse> {
        let cmd = DeleteCommand::delete_object(aid);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Delete an object and related objects
    pub fn delete_object_and_related(&mut self, aid: &[u8]) -> Result<DeleteResponse> {
        let cmd = DeleteCommand::delete_object_and_related(aid);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Get the status of applications
    pub fn get_applications_status(&mut self) -> Result<GetStatusResponse> {
        let cmd = GetStatusCommand::all_with_type(get_status_p1::APPLICATIONS);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Get the status of load files
    pub fn get_load_files_status(&mut self) -> Result<GetStatusResponse> {
        let cmd = GetStatusCommand::all_with_type(get_status_p1::EXEC_LOAD_FILES);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Install a package for load
    pub fn install_for_load(
        &mut self,
        package_aid: &[u8],
        security_domain_aid: Option<&[u8]>,
    ) -> Result<InstallResponse> {
        // Use ISD if no security domain AID provided
        let sd_aid = security_domain_aid.unwrap_or(SECURITY_DOMAIN_AID);

        let cmd = InstallCommand::for_load(package_aid, sd_aid);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Install for install and make selectable
    pub fn install_for_install_and_make_selectable(
        &mut self,
        package_aid: &[u8],
        applet_aid: &[u8],
        instance_aid: &[u8],
        params: &[u8],
    ) -> Result<InstallResponse> {
        // Use empty privileges
        let privileges = &[0x00];

        let cmd = InstallCommand::for_install_and_make_selectable(
            package_aid,
            applet_aid,
            instance_aid,
            privileges,
            params,
            &[] as &[u8], // Empty token
        );

        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Load a CAP file
    pub fn load_cap_file<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
        mut callback: Option<&mut dyn FnMut(usize, usize) -> Result<()>>,
    ) -> Result<()> {
        // Create load command stream
        let mut stream = LoadCommandStream::from_file(path)?;

        // Process each block
        while stream.has_next() {
            // Get next block
            let (is_last, block_number, block_data) = stream
                .next_block()
                .ok_or(Error::Other("Unexpected end of data"))?;

            // Create LOAD command
            let p1 = if is_last {
                load_p1::LAST_BLOCK
            } else {
                load_p1::MORE_BLOCKS
            };
            let cmd = LoadCommand::with_block_data(p1, block_number, block_data.to_vec());

            // Execute command
            let response = self.executor.execute(&cmd)?;

            // Call callback if provided
            if let Some(cb) = &mut callback {
                cb(stream.current_block(), stream.blocks_count())?;
            }

            // Check response
            if !matches!(response, LoadResponse::Success) {
                return Err(Error::Other("Load failed"));
            }
        }

        Ok(())
    }

    /// Extract information from a CAP file without loading it
    pub fn analyze_cap_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<CapFileInfo> {
        LoadCommandStream::extract_info(path)
    }

    /// Get the executor
    pub fn executor(&self) -> &E {
        &self.executor
    }

    /// Get a mutable reference to the executor
    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    /// Get the session
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Close the secure channel
    pub fn close_secure_channel(&mut self) -> Result<()> {
        // Reset the executor (will drop any secure channel processors)
        self.executor.reset().map_err(Error::from)?;
        self.session = None;
        Ok(())
    }

    /// Get the last response
    pub fn last_response(&self) -> Option<&[u8]> {
        self.last_response.as_ref().map(|b| b.as_ref())
    }

    /// Get card data including CPLC information
    pub fn get_card_data(&mut self) -> Result<Bytes> {
        // If we don't have a secure channel, we need to open one
        if self.session.is_none() {
            self.open_secure_channel()?;
        }

        // Simple GET DATA command for card data
        let get_data_cmd = Command::new(0x80, 0xCA, 0x00, 0x66).with_le(0x00);

        // Execute and get the response
        let response = self.executor.transmit(&get_data_cmd.to_bytes())?;

        // Check if the command was successful
        if response.len() >= 2 {
            let sw = StatusWord::new(response[response.len() - 2], response[response.len() - 1]);
            if sw.is_success() {
                return Ok(Bytes::copy_from_slice(&response[..response.len() - 2]));
            } else {
                return Err(Error::CardStatus(sw));
            }
        }

        Err(Error::Other("Invalid response"))
    }

    /// Personalize a card application by storing data
    pub fn personalize_application(&mut self, app_aid: &[u8], data: &[u8]) -> Result<()> {
        // Create INSTALL [for personalization] command
        let cmd = InstallCommand::for_personalization(app_aid, data);

        // Execute the command
        let response = self.executor.execute(&cmd)?;

        // Check if successful
        if !matches!(response, InstallResponse::Success) {
            return Err(Error::Other("Personalization failed"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apdu_core::CardExecutor;
    use hex_literal::hex;

    // Custom mock transport for tests
    #[derive(Debug)]
    struct TestTransport {
        responses: Vec<Bytes>,
    }

    impl TestTransport {
        fn new() -> Self {
            Self {
                responses: Vec::new(),
            }
        }

        fn add_response(&mut self, response: Bytes) {
            self.responses.push(response);
        }
    }

    impl apdu_core::transport::CardTransport for TestTransport {
        fn do_transmit_raw(
            &mut self,
            _command: &[u8],
        ) -> std::result::Result<Bytes, apdu_core::transport::error::TransportError> {
            if self.responses.is_empty() {
                return Err(apdu_core::transport::error::TransportError::Transmission);
            }

            if self.responses.len() == 1 {
                Ok(self.responses[0].clone())
            } else {
                Ok(self.responses.remove(0))
            }
        }

        fn is_connected(&self) -> bool {
            true
        }

        fn reset(
            &mut self,
        ) -> std::result::Result<(), apdu_core::transport::error::TransportError> {
            Ok(())
        }
    }

    // Mock response for select AID
    fn mock_select_response() -> Bytes {
        Bytes::copy_from_slice(&hex!(
            "6F 10 84 08 A0 00 00 01 51 00 00 00 A5 04 9F 65 01 FF 90 00"
        ))
    }

    // Mock response for initialize update
    fn mock_init_update_response() -> Bytes {
        Bytes::copy_from_slice(&hex!(
            "00 00 02 65 01 83 03 95 36 62 20 02 00 0D E9 C6 2B A1 C4 C8 E5 5F CB 91 B6 65 90 00"
        ))
    }

    // Mock response for external authenticate
    fn mock_ext_auth_response() -> Bytes {
        Bytes::copy_from_slice(&hex!("90 00"))
    }

    #[test]
    fn test_select_card_manager() {
        // Create a mock transport
        let mut transport = TestTransport::new();
        transport.add_response(mock_select_response());

        // Create executor with the transport
        let executor = CardExecutor::new(transport);

        // Create GlobalPlatform instance
        let mut gp = GlobalPlatform::new(executor);

        // Try to select card manager
        let result = gp.select_card_manager();
        assert!(result.is_ok());

        // Validate the response
        let response = result.unwrap();
        assert!(response.is_success());
    }

    #[test]
    fn test_delete_object() {
        // Create a test transport with success response
        let mut transport = TestTransport::new();
        transport.add_response(Bytes::copy_from_slice(&hex!("9000")));

        // Create executor
        let executor = CardExecutor::new(transport);

        // Create GlobalPlatform instance
        let mut gp = GlobalPlatform::new(executor);

        // Try to delete an object without secure channel (should fail)
        let aid = hex!("A0000000030000");
        let result = gp.delete_object(&aid);
        assert!(result.is_err());
    }
}
