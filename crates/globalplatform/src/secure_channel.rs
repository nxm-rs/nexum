//! Secure channel implementation for GlobalPlatform
//!
//! This module provides the SecureChannel type that wraps card communication
//! with SCP02 security.

use std::fmt;

use bytes::{BufMut, BytesMut};
use cipher::{Iv, Key};
use nexum_apdu_core::processor::secure::SecureChannel;
use nexum_apdu_core::processor::{CommandProcessor, error::ProcessorError};
use nexum_apdu_core::transport::CardTransport;
use nexum_apdu_core::{ApduCommand, Command, Response};
use rand::RngCore;
use tracing::{debug, trace, warn};

use crate::crypto::{HostChallenge, Scp02};
use crate::{
    commands::{
        ExternalAuthenticateCommand, ExternalAuthenticateResponse, InitializeUpdateCommand,
        InitializeUpdateResponse,
    },
    crypto::{encrypt_icv, mac_full_3des},
    session::{Keys, Session},
};

/// SCP02 command wrapper
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct SCP02Wrapper {
    /// MAC key
    mac_key: Key<Scp02>,
    /// Initial chaining vector
    icv: Iv<Scp02>,
}

impl SCP02Wrapper {
    /// Create a new SCP02 wrapper with the specified MAC key
    pub fn new(key: Key<Scp02>) -> crate::Result<Self> {
        Ok(Self {
            mac_key: key,
            icv: Default::default(),
        })
    }

    /// Wrap an APDU command by adding a MAC
    pub fn wrap_command(&mut self, command: &Command) -> crate::Result<Command> {
        // Prepare data for MAC calculation
        let mut mac_data = BytesMut::with_capacity(5 + command.data().map_or(0, |d| d.len()));

        // Set CLA byte with secure messaging bit
        let cla = command.class() | 0x04;
        mac_data.put_u8(cla);
        mac_data.put_u8(command.instruction());
        mac_data.put_u8(command.p1());
        mac_data.put_u8(command.p2());

        // Lc is data length + 8 (for MAC)
        let data_len = command.data().map_or(0, |d| d.len());
        mac_data.put_u8((data_len + 8) as u8);

        // Add command data
        if let Some(data) = command.data() {
            mac_data.put_slice(data);
        }

        // Encrypt the ICV if it's not default
        let icv_for_mac = if self.icv == Default::default() {
            self.icv
        } else {
            encrypt_icv(&self.mac_key, &self.icv)
        };

        // Calculate the MAC
        let mac = mac_full_3des(&self.mac_key, &icv_for_mac, &mac_data);

        // Save MAC as ICV for next command
        self.icv.copy_from_slice(&mac);

        // Create new command with MAC appended
        let mut new_data = BytesMut::with_capacity(data_len + 8);
        if let Some(data) = command.data() {
            new_data.put_slice(data);
        }
        new_data.put_slice(&mac);

        // Create new command
        let mut new_cmd = Command::new(cla, command.instruction(), command.p1(), command.p2());

        new_cmd = new_cmd.with_data(new_data.freeze());

        // Set Le if original command had it
        if let Some(le) = command.expected_length() {
            new_cmd = new_cmd.with_le(le);
        }

        Ok(new_cmd)
    }

    /// Get the current ICV
    pub const fn icv(&self) -> &Iv<Scp02> {
        &self.icv
    }

    /// Encrypt the ICV for the next operation
    pub fn encrypt_icv(&mut self) -> crate::Result<()> {
        let encrypted = encrypt_icv(&self.mac_key, &self.icv);
        self.icv.copy_from_slice(&encrypted);
        Ok(())
    }
}

/// GPSecureChannel implements the CommandProcessor and SecureChannel traits for SCP02
#[derive(Clone)]
pub struct GPSecureChannel {
    /// Session containing keys and state
    session: Session,
    /// Command wrapper for SCP02
    wrapper: SCP02Wrapper,
    /// Whether the channel is established
    established: bool,
}

impl fmt::Debug for GPSecureChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GPSecureChannel")
            .field("established", &self.established)
            .finish()
    }
}

impl GPSecureChannel {
    /// Create a new secure channel with the specified session
    pub fn new(session: Session) -> crate::Result<Self> {
        let wrapper = SCP02Wrapper::new(*session.keys().mac())?;

        Ok(Self {
            session,
            wrapper,
            established: false,
        })
    }

    /// Get a reference to the session
    pub const fn session(&self) -> &Session {
        &self.session
    }

    /// Authenticate the secure channel using EXTERNAL AUTHENTICATE
    pub fn authenticate(
        &mut self,
        transport: &mut dyn CardTransport,
    ) -> Result<(), ProcessorError> {
        // Create EXTERNAL AUTHENTICATE command
        let auth_cmd = ExternalAuthenticateCommand::from_challenges(
            self.session.keys().enc(),
            self.session.sequence_counter(),
            self.session.card_challenge(),
            self.session.host_challenge(),
        )
        .map_err(|_| {
            ProcessorError::secure_messaging("Failed to create EXTERNAL AUTHENTICATE command")
        })?;

        // Convert to Command
        let command = auth_cmd.to_command();

        // Wrap the command with MAC
        let wrapped_cmd = self.wrapper.wrap_command(&command).map_err(|_| {
            ProcessorError::secure_messaging("Failed to wrap EXTERNAL AUTHENTICATE command")
        })?;

        // Send wrapped command
        let response_bytes = transport
            .transmit_raw(&wrapped_cmd.to_bytes())
            .map_err(ProcessorError::from)?;

        // Parse response
        let auth_response =
            ExternalAuthenticateResponse::from_bytes(&response_bytes).map_err(|_| {
                ProcessorError::invalid_response("Failed to parse EXTERNAL AUTHENTICATE response")
            })?;

        // Check if successful
        if !matches!(auth_response, ExternalAuthenticateResponse::Success) {
            self.established = false;
            return Err(ProcessorError::authentication_failed(
                "EXTERNAL AUTHENTICATE failed",
            ));
        }

        // Mark channel as established
        self.established = true;

        Ok(())
    }
}

impl CommandProcessor for GPSecureChannel {
    fn do_process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError> {
        if !self.established {
            return Err(ProcessorError::session("Secure channel not established"));
        }

        trace!(command = ?command, "Processing command with GlobalPlatform SCP02");

        // Wrap the command with SCP02 security
        let wrapped_cmd = self
            .wrapper
            .wrap_command(command)
            .map_err(|_| ProcessorError::secure_messaging("Failed to wrap command"))?;

        trace!(wrapped = ?wrapped_cmd, "Command wrapped with MAC");

        // Send the wrapped command
        let response_bytes = transport
            .transmit_raw(&wrapped_cmd.to_bytes())
            .map_err(ProcessorError::from)?;

        trace!(response = ?hex::encode(&response_bytes), "Received response");

        // For SCP02, we don't need to unwrap the response - just parse it into a Response object
        let response = Response::from_bytes(&response_bytes)
            .map_err(|_| ProcessorError::InvalidResponse("Failed to parse response"))?;

        Ok(response)
    }

    fn is_active(&self) -> bool {
        self.established
    }
}

impl SecureChannel for GPSecureChannel {
    fn is_established(&self) -> bool {
        self.established
    }

    fn close(&mut self) -> Result<(), ProcessorError> {
        debug!("Closing GlobalPlatform SCP02 secure channel");
        self.established = false;
        Ok(())
    }

    fn reestablish(&mut self) -> Result<(), ProcessorError> {
        warn!("Reestablish not implemented for GlobalPlatform SCP02");
        Err(ProcessorError::session(
            "Cannot reestablish GlobalPlatform SCP02 channel - a new session must be created",
        ))
    }
}

/// Secure channel provider for GlobalPlatform SCP02
#[derive(Debug, Clone)]
pub struct GPSecureChannelProvider {
    /// Keys for this secure channel
    keys: Keys,
}

impl GPSecureChannelProvider {
    /// Create a new SCP02 secure channel provider with the given keys
    pub const fn new(keys: Keys) -> Self {
        Self { keys }
    }
}

/// Create a secure channel provider from a session
pub const fn create_secure_channel_provider(keys: Keys) -> GPSecureChannelProvider {
    GPSecureChannelProvider::new(keys)
}

impl nexum_apdu_core::processor::secure::SecureChannelProvider for GPSecureChannelProvider {
    fn create_secure_channel(
        &self,
        transport: &mut dyn CardTransport,
    ) -> Result<Box<dyn CommandProcessor>, ProcessorError> {
        // Generate host challenge
        let mut host_challenge = HostChallenge::default();
        rand::rng().fill_bytes(&mut host_challenge);

        // Step 1: Send INITIALIZE UPDATE
        let init_cmd = InitializeUpdateCommand::with_challenge(host_challenge.to_vec());
        let response_bytes = transport
            .transmit_raw(&init_cmd.to_bytes())
            .map_err(ProcessorError::from)?;

        // Parse response
        let init_response =
            InitializeUpdateResponse::from_bytes(&response_bytes).map_err(|_| {
                ProcessorError::invalid_response("Failed to parse INITIALIZE UPDATE response")
            })?;

        // Check for successful response
        if !matches!(init_response, InitializeUpdateResponse::Success { .. }) {
            return Err(ProcessorError::authentication_failed(
                "INITIALIZE UPDATE failed",
            ));
        }

        match init_response {
            InitializeUpdateResponse::Success { .. } => {
                // Create session directly from response
                let session = Session::from_response(&self.keys, &init_response, host_challenge)
                    .map_err(|_| {
                        ProcessorError::authentication_failed("Failed to create session")
                    })?;

                // Create secure channel with session (not yet established)
                let mut channel = GPSecureChannel::new(session)
                    .map_err(|_| ProcessorError::session("Failed to create secure channel"))?;

                // Step 2: Authenticate the channel (sends EXTERNAL AUTHENTICATE)
                channel.authenticate(transport)?;

                Ok(Box::new(channel))
            }
            _ => Err(ProcessorError::authentication_failed(
                "INITIALIZE UPDATE failed",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Keys;
    use bytes::Bytes;
    use hex_literal::hex;
    use nexum_apdu_core::processor::secure::SecureChannelProvider;

    // Create a mock transport implementation for testing
    #[derive(Debug)]
    struct TestMockTransport {
        commands: Vec<Vec<u8>>,
        responses: Vec<Bytes>,
    }

    impl TestMockTransport {
        fn new() -> Self {
            Self {
                commands: Vec::new(),
                responses: Vec::new(),
            }
        }

        fn with_response(response: Vec<u8>) -> Self {
            let mut transport = Self::new();
            transport.responses.push(Bytes::from(response));
            transport
        }
    }

    impl CardTransport for TestMockTransport {
        fn do_transmit_raw(
            &mut self,
            command: &[u8],
        ) -> Result<Bytes, nexum_apdu_core::transport::error::TransportError> {
            self.commands.push(command.to_vec());

            if self.responses.is_empty() {
                return Err(nexum_apdu_core::transport::error::TransportError::Transmission);
            }

            // Either return the next response or keep reusing the last one
            if self.responses.len() == 1 {
                Ok(self.responses[0].clone())
            } else {
                Ok(self.responses.remove(0))
            }
        }

        fn is_connected(&self) -> bool {
            true
        }

        fn reset(&mut self) -> Result<(), nexum_apdu_core::transport::error::TransportError> {
            self.commands.clear();
            Ok(())
        }
    }

    // Helper to create a test session with realistic data
    fn create_test_session() -> Session {
        // Realistic test values based on actual card responses
        let key = Key::<Scp02>::from_slice(hex!("404142434445464748494a4b4c4d4e4f").as_slice());
        let keys = Keys::from_single_key(*key);
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000");
        let host_challenge = hex!("f0467f908e5ca23f");

        let response = InitializeUpdateResponse::from_bytes(&init_response).unwrap();
        Session::from_response(&keys, &response, host_challenge).unwrap()
    }

    // A test-specific secure channel provider that uses a fixed host challenge
    #[derive(Debug, Clone)]
    struct TestGPSecureChannelProvider {
        keys: Keys,
    }

    impl TestGPSecureChannelProvider {
        fn new(keys: Keys) -> Self {
            Self { keys }
        }
    }

    impl SecureChannelProvider for TestGPSecureChannelProvider {
        fn create_secure_channel(
            &self,
            transport: &mut dyn CardTransport,
        ) -> Result<Box<dyn CommandProcessor>, ProcessorError> {
            // Use a fixed host challenge for testing instead of random
            let host_challenge = hex!("f0467f908e5ca23f");

            // Step 1: Send INITIALIZE UPDATE
            let init_cmd = InitializeUpdateCommand::with_challenge(host_challenge.to_vec());
            let response_bytes = transport
                .transmit_raw(&init_cmd.to_bytes())
                .map_err(ProcessorError::from)?;

            // Parse response
            let init_response =
                InitializeUpdateResponse::from_bytes(&response_bytes).map_err(|_| {
                    ProcessorError::invalid_response("Failed to parse INITIALIZE UPDATE response")
                })?;

            // Check for successful response
            if !matches!(init_response, InitializeUpdateResponse::Success { .. }) {
                return Err(ProcessorError::authentication_failed(
                    "INITIALIZE UPDATE failed",
                ));
            }

            // Create session from response
            let session = Session::from_response(&self.keys, &init_response, host_challenge)
                .map_err(|_| ProcessorError::authentication_failed("Failed to create session"))?;

            // Create secure channel with session (not yet established)
            let mut channel = GPSecureChannel::new(session)
                .map_err(|_| ProcessorError::session("Failed to create secure channel"))?;

            // Step 2: Authenticate the channel (sends EXTERNAL AUTHENTICATE)
            channel.authenticate(transport)?;

            Ok(Box::new(channel))
        }
    }

    #[test]
    fn test_wrap_command() {
        let mac_key = Key::<Scp02>::from_slice(hex!("2983ba77d709c2daa1e6000abccac951").as_slice());
        let mut wrapper = SCP02Wrapper::new(*mac_key).unwrap();

        // Verify initial ICV
        assert_eq!(wrapper.icv(), &Iv::<Scp02>::default());

        // Test wrapping a command
        let data = hex!("1d4de92eaf7a2c9f");
        let cmd = Command::new_with_data(0x80, 0x82, 0x01, 0x00, data.to_vec());

        let wrapped_cmd = wrapper.wrap_command(&cmd).unwrap();
        let wrapped_bytes = wrapped_cmd.to_bytes();

        assert_eq!(
            wrapped_bytes.to_vec().as_slice(),
            hex!("84820100101d4de92eaf7a2c9f8f9b0df681c1d3ec")
        );

        // Verify ICV is updated
        let iv = Iv::<Scp02>::from_slice(hex!("8f9b0df681c1d3ec").as_slice());
        assert_eq!(wrapper.icv(), iv);

        // Test wrapping another command
        let data = hex!("4f00");
        let mut cmd = Command::new_with_data(0x80, 0xF2, 0x80, 0x02, data.to_vec());
        cmd = cmd.with_le(0);

        let wrapped_cmd = wrapper.wrap_command(&cmd).unwrap();
        let wrapped_bytes = wrapped_cmd.to_bytes();

        assert_eq!(
            wrapped_bytes.to_vec().as_slice(),
            hex!("84f280020a4f0030f149209e17b39700")
        );
    }

    #[test]
    fn test_secure_channel_processor() {
        // Create mock transport
        let mut transport = TestMockTransport::with_response(hex!("9000").to_vec());

        // Create test session
        let session = create_test_session();

        // Create secure channel
        let mut channel = GPSecureChannel::new(session).unwrap();

        // Mark it as established for the test
        channel.established = true;

        // Create a simple command
        let command = Command::new(0x80, 0xCA, 0x00, 0x00).with_le(0);

        // Process the command
        let result = channel.process_command(&command, &mut transport);

        // Verify command was processed successfully
        assert!(result.is_ok());

        // Verify CLA byte was modified in the sent command
        assert_eq!(transport.commands[0][0], 0x84); // MAC bit set

        // Verify the secure channel can be closed
        assert!(channel.close().is_ok());
        assert!(!channel.is_established());

        // Verify a new command would fail after closing
        let result = channel.process_command(&command, &mut transport);
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate() {
        // Create mock transport that returns success for auth command
        let mut transport = TestMockTransport::with_response(hex!("9000").to_vec());

        // Create test session
        let session = create_test_session();

        // Create secure channel (not established yet)
        let mut channel = GPSecureChannel::new(session).unwrap();

        // Call authenticate
        let result = channel.authenticate(&mut transport);

        assert!(result.is_ok());
        assert!(channel.is_established());

        // Verify EXTERNAL AUTHENTICATE command was sent
        assert!(!transport.commands.is_empty());
        assert_eq!(transport.commands[0][1], 0x82); // INS for EXTERNAL AUTHENTICATE
        assert_eq!(transport.commands[0][0], 0x84); // CLA with SECURE bit set
    }

    #[test]
    fn test_secure_channel_provider() {
        // Create mock transport with predetermined responses
        let mut transport = TestMockTransport::new();

        // Response to INITIALIZE UPDATE - use the same one that worked in create_test_session()
        transport.responses.push(Bytes::copy_from_slice(&hex!(
            "000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000"
        )));

        // Response to EXTERNAL AUTHENTICATE
        transport
            .responses
            .push(Bytes::copy_from_slice(&hex!("9000")));

        // Create test keys - use the same keys as in create_test_session()
        let key = Key::<Scp02>::from_slice(hex!("404142434445464748494a4b4c4d4e4f").as_slice());
        let keys = Keys::from_single_key(*key);

        // Create our test-specific provider that uses fixed host challenge
        let provider = TestGPSecureChannelProvider::new(keys);

        // Create secure channel
        let channel_result = provider.create_secure_channel(&mut transport);

        // This should now pass since we're using a deterministic approach
        assert!(channel_result.is_ok());

        // Verify INITIALIZE UPDATE command was sent
        assert!(!transport.commands.is_empty());
        assert_eq!(transport.commands[0][1], 0x50); // INS for INITIALIZE UPDATE

        // Verify EXTERNAL AUTHENTICATE command was sent with MAC bit set
        assert!(transport.commands.len() >= 2); // Make sure we have at least 2 commands
        assert_eq!(transport.commands[1][0], 0x84); // CLA with MAC bit set
        assert_eq!(transport.commands[1][1], 0x82); // INS for EXTERNAL AUTHENTICATE
    }
}
