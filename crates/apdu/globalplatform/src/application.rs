//! GlobalPlatform application implementation
//!
//! This module provides the main GlobalPlatform application interface,
//! which encapsulates all the functionality for managing smart cards.

use std::path::Path;

use nexum_apdu_core::prelude::*;

use crate::commands::delete::DeleteOk;
use crate::commands::get_status::GetStatusOk;
use crate::commands::install::InstallOk;
use crate::commands::select::SelectOk;
use crate::commands::{
    DeleteCommand, GetStatusCommand, InstallCommand, LoadCommand, SelectCommand,
};
use crate::constants::{SECURITY_DOMAIN_AID, get_status_p1, load_p1};
use crate::error::{Error, Result};
use crate::load::{CapFileInfo, LoadCommandStream};
use crate::session::Session;

/// GlobalPlatform card management application
#[allow(missing_debug_implementations)]
pub struct GlobalPlatform<E>
where
    E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
{
    /// Card executor
    executor: E,
    /// Current session
    session: Option<Session>,
    /// Last response for session creation
    last_response: Option<Bytes>,
}

impl<E> GlobalPlatform<E>
where
    E: Executor + ResponseAwareExecutor + SecureChannelExecutor,
{
    /// Create a new GlobalPlatform instance
    pub const fn new(executor: E) -> Self {
        Self {
            executor,
            session: None,
            last_response: None,
        }
    }

    /// Select the card manager (ISD)
    pub fn select_card_manager(&mut self) -> Result<SelectOk> {
        self.select_application(SECURITY_DOMAIN_AID)
    }

    /// Select an application by AID
    pub fn select_application(&mut self, aid: &[u8]) -> Result<SelectOk> {
        // Create SELECT command
        let cmd = SelectCommand::with_aid(aid.to_vec());

        // Execute command using typed execution flow
        let response = self.executor.execute(&cmd).map_err(Error::from)?;

        // Store response for possible later use
        if let Ok(raw_response) = self.executor.last_response() {
            self.last_response = Some(Bytes::copy_from_slice(raw_response));
        }

        Ok(response)
    }

    /// Delete an object
    pub fn delete_object(&mut self, aid: &[u8]) -> Result<DeleteOk> {
        let cmd = DeleteCommand::delete_object(aid);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Delete an object and related objects
    pub fn delete_object_and_related(&mut self, aid: &[u8]) -> Result<DeleteOk> {
        let cmd = DeleteCommand::delete_object_and_related(aid);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Get the status of applications
    pub fn get_applications_status(&mut self) -> Result<GetStatusOk> {
        let cmd = GetStatusCommand::all_with_type(get_status_p1::APPLICATIONS);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Get the status of load files
    pub fn get_load_files_status(&mut self) -> Result<GetStatusOk> {
        let cmd = GetStatusCommand::all_with_type(get_status_p1::EXEC_LOAD_FILES);
        self.executor.execute(&cmd).map_err(Error::from)
    }

    /// Install a package for load
    pub fn install_for_load(
        &mut self,
        package_aid: &[u8],
        security_domain_aid: Option<&[u8]>,
    ) -> Result<InstallOk> {
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
    ) -> Result<InstallOk> {
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
                .ok_or_else(|| Error::other("Unexpected end of data"))?;

            // Create LOAD command
            let p1 = if is_last {
                load_p1::LAST_BLOCK
            } else {
                load_p1::MORE_BLOCKS
            };
            let cmd = LoadCommand::with_block_data(p1, block_number, block_data.to_vec());

            // Execute command
            self.executor
                .execute(&cmd)
                .map_err(|e| Error::from(e).with_context("Load failed"))?;

            // Call callback if provided
            if let Some(cb) = &mut callback {
                cb(stream.current_block(), stream.blocks_count())?;
            }
        }

        Ok(())
    }

    /// Install a specific applet from a CAP file
    pub fn install_applet_from_cap<P: AsRef<Path>>(
        &mut self,
        cap_file: P,
        applet_index: usize,
        callback: Option<&mut dyn FnMut(usize, usize) -> Result<()>>,
    ) -> Result<()> {
        // Extract CAP file info
        let info = LoadCommandStream::extract_info(&cap_file)?;

        let package_aid = info
            .package_aid
            .ok_or(Error::CapFile("Package AID not found"))?;

        if applet_index >= info.applet_aids.len() {
            return Err(Error::CapFile("Invalid applet index"));
        }

        let applet_aid = &info.applet_aids[applet_index];

        // First, install the package
        self.install_for_load(&package_aid, None)?;

        // Then load the CAP file
        self.load_cap_file(cap_file, callback)?;

        // Finally, install and make selectable
        self.install_for_install_and_make_selectable(
            &package_aid,
            applet_aid,
            applet_aid, // using same AID for instance
            &[],        // empty params
        )?;

        Ok(())
    }

    /// Install all applets from a CAP file
    pub fn install_all_applets_from_cap<P: AsRef<Path>>(
        &mut self,
        cap_file: P,
        callback: Option<&mut dyn FnMut(usize, usize) -> Result<()>>,
    ) -> Result<()> {
        // Extract CAP file info
        let info = LoadCommandStream::extract_info(&cap_file)?;

        let package_aid = info
            .package_aid
            .ok_or(Error::CapFile("Package AID not found"))?;

        if info.applet_aids.is_empty() {
            return Err(Error::CapFile("No applets found in CAP file"));
        }

        // First, install the package
        self.install_for_load(&package_aid, None)?;

        // Then load the CAP file
        self.load_cap_file(&cap_file, callback)?;

        // Finally, install and make selectable each applet
        for applet_aid in &info.applet_aids {
            // Use the applet AID as the instance AID as well
            self.install_for_install_and_make_selectable(
                &package_aid,
                applet_aid,
                applet_aid, // using same AID for instance
                &[],        // empty params
            )?;
        }

        Ok(())
    }

    /// Extract information from a CAP file without loading it
    pub fn analyze_cap_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<CapFileInfo> {
        LoadCommandStream::extract_info(path)
    }

    /// Get the executor
    pub const fn executor(&self) -> &E {
        &self.executor
    }

    /// Get a mutable reference to the executor
    pub const fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    /// Get the session
    pub const fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Close the secure channel
    pub fn close_secure_channel(&mut self) -> Result<()> {
        // Close the secure channel through the executor
        self.executor.close_secure_channel().map_err(Error::from)?;
        self.session = None;
        Ok(())
    }

    /// Open a secure channel using default keys
    pub fn open_secure_channel(&mut self) -> Result<()> {
        // // First, reset any existing channel
        // self.close_secure_channel()?;

        // Select the card manager (ISD) first
        self.select_card_manager()?;

        // Open the secure channel through the executor
        self.executor.open_secure_channel().map_err(Error::from)
    }

    /// Check if the secure channel is currently established
    pub fn is_secure_channel_open(&self) -> bool {
        self.executor.has_secure_channel()
    }

    /// Get the current security level of the secure channel
    pub fn security_level(&self) -> SecurityLevel {
        self.executor.security_level()
    }

    /// Get the last response
    pub fn last_response(&self) -> Option<&[u8]> {
        self.last_response.as_ref().map(|b| b.as_ref())
    }

    /// Get card data including CPLC information
    pub fn get_card_data(&mut self) -> Result<Bytes> {
        // Simple GET DATA command for card data
        let get_data_cmd = Command::new(0x80, 0xCA, 0x00, 0x66).with_le(0x00);

        // Execute and get the response
        let response = self
            .executor
            .transmit_raw(&get_data_cmd.to_bytes())
            .map_err(Error::from)?;

        // Check if the command was successful
        if response.len() >= 2 {
            let sw = StatusWord::new(response[response.len() - 2], response[response.len() - 1]);
            if sw.is_success() {
                return Ok(Bytes::copy_from_slice(&response[..response.len() - 2]));
            } else {
                return Err(Error::CardStatus(sw));
            }
        }

        Err(Error::other("Invalid response"))
    }

    /// Personalize a card application by storing data
    pub fn personalize_application(&mut self, app_aid: &[u8], data: &[u8]) -> Result<()> {
        // Create INSTALL [for personalization] command
        let cmd = InstallCommand::for_personalization(app_aid, data);

        // Execute the command
        self.executor
            .execute(&cmd)
            .map_err(|e| Error::from(e).with_context("Personalization failed"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    impl nexum_apdu_core::transport::CardTransport for TestTransport {
        fn transmit_raw(
            &mut self,
            _command: &[u8],
        ) -> std::result::Result<Bytes, nexum_apdu_core::Error> {
            if self.responses.is_empty() {
                return Err(nexum_apdu_core::Error::other("No response available"));
            }

            if self.responses.len() == 1 {
                Ok(self.responses[0].clone())
            } else {
                Ok(self.responses.remove(0))
            }
        }

        fn reset(&mut self) -> std::result::Result<(), nexum_apdu_core::Error> {
            Ok(())
        }
    }

    // Mock response for select AID
    fn mock_select_response() -> Bytes {
        Bytes::copy_from_slice(&hex!(
            "6F 10 84 08 A0 00 00 01 51 00 00 00 A5 04 9F 65 01 FF 90 00"
        ))
    }

    #[test]
    fn test_select_card_manager() {
        // Create a mock transport
        let mut transport = TestTransport::new();
        transport.add_response(mock_select_response());

        // Create secure channel with the mock transport
        let secure_channel = crate::GPSecureChannel::new(transport, crate::Keys::default());

        // Create executor with the secure channel
        let executor = CardExecutor::new(secure_channel);

        // Create GlobalPlatform instance
        let mut gp = GlobalPlatform::new(executor);

        // Try to select card manager
        let result = gp.select_card_manager();
        assert!(result.is_ok());
    }
}
