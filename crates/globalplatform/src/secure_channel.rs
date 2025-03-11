//! Secure channel implementation for GlobalPlatform
//!
//! This module provides the SecureChannel type that wraps card communication
//! with SCP02 security.

use std::fmt;

use apdu_core::processor::secure::{SecureChannel, SecurityLevel};
use apdu_core::processor::{CommandProcessor, error::ProcessorError};
use apdu_core::transport::CardTransport;
use apdu_core::{ApduCommand, Command, Response};
use bytes::{BufMut, BytesMut};
use tracing::{debug, trace, warn};

use crate::{
    Error,
    crypto::{NULL_BYTES_8, encrypt_icv, mac_full_3des},
    session::Session,
};

/// SCP02 command wrapper
#[derive(Clone)]
pub struct SCP02Wrapper {
    /// MAC key
    mac_key: [u8; 16],
    /// Initial chaining vector
    icv: [u8; 8],
    /// Security level
    security_level: SecurityLevel,
}

impl SCP02Wrapper {
    /// Create a new SCP02 wrapper with the specified MAC key
    pub fn new(mac_key: &[u8], security_level: SecurityLevel) -> crate::Result<Self> {
        let mut key = [0u8; 16];
        if mac_key.len() != 16 {
            return Err(Error::InvalidLength {
                expected: 16,
                actual: mac_key.len(),
            });
        }
        key.copy_from_slice(mac_key);

        Ok(Self {
            mac_key: key,
            icv: NULL_BYTES_8,
            security_level,
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

        // Calculate the MAC
        let icv = if self.icv == NULL_BYTES_8 {
            &NULL_BYTES_8
        } else {
            &self.icv
        };

        let mac = mac_full_3des(&self.mac_key, &mac_data, icv)?;
        if mac.len() != 8 {
            return Err(Error::InvalidLength {
                expected: 8,
                actual: mac.len(),
            });
        }

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
    pub fn icv(&self) -> &[u8] {
        &self.icv
    }

    /// Get the security level
    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    /// Encrypt the ICV for the next operation
    pub fn encrypt_icv(&mut self) -> crate::Result<()> {
        let encrypted = encrypt_icv(&self.mac_key, &self.icv)?;
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
    pub fn new(session: Session, security_level: SecurityLevel) -> crate::Result<Self> {
        let wrapper = SCP02Wrapper::new(session.keys().mac(), security_level)?;

        Ok(Self {
            session,
            wrapper,
            established: true,
        })
    }

    /// Get a reference to the session
    pub fn session(&self) -> &Session {
        &self.session
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

    fn security_level(&self) -> SecurityLevel {
        self.wrapper.security_level()
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
    /// Session for this secure channel
    session: Session,
}

impl GPSecureChannelProvider {
    /// Create a new SCP02 secure channel provider with the given session
    pub fn new(session: Session) -> Self {
        Self { session }
    }
}

/// Create a secure channel provider from a session
pub fn create_secure_channel_provider(session: Session) -> GPSecureChannelProvider {
    GPSecureChannelProvider::new(session)
}

impl apdu_core::processor::secure::SecureChannelProvider for GPSecureChannelProvider {
    fn create_secure_channel(
        &self,
        _transport: &mut dyn CardTransport,
        level: SecurityLevel,
    ) -> Result<Box<dyn CommandProcessor>, ProcessorError> {
        // Create the secure channel
        let channel = match GPSecureChannel::new(self.session.clone(), level) {
            Ok(channel) => channel,
            Err(_) => {
                return Err(ProcessorError::session("Failed to create secure channel"));
            }
        };

        Ok(Box::new(channel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Keys;
    use apdu_core::processor::secure::SecureChannelProvider;
    use bytes::Bytes;
    use hex_literal::hex;

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
        ) -> Result<Bytes, apdu_core::transport::error::TransportError> {
            self.commands.push(command.to_vec());

            if self.responses.is_empty() {
                return Err(apdu_core::transport::error::TransportError::Transmission);
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

        fn reset(&mut self) -> Result<(), apdu_core::transport::error::TransportError> {
            self.commands.clear();
            Ok(())
        }
    }

    // Helper to create a test session with realistic data
    fn create_test_session() -> Session {
        // Realistic test values based on actual card responses
        let keys = Keys::from_single_key(hex!("404142434445464748494a4b4c4d4e4f"));
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000");
        let host_challenge = hex!("f0467f908e5ca23f");

        Session::new(&keys, &init_response, &host_challenge).unwrap()
    }

    #[test]
    fn test_wrap_command() {
        let mac_key = hex!("2983ba77d709c2daa1e6000abccac951");
        let mut wrapper = SCP02Wrapper::new(&mac_key, SecurityLevel::MACProtection).unwrap();

        // Verify initial ICV
        assert_eq!(wrapper.icv(), NULL_BYTES_8);

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
        assert_eq!(wrapper.icv(), hex!("8f9b0df681c1d3ec"));

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
        let mut channel = GPSecureChannel::new(session, SecurityLevel::MACProtection).unwrap();

        // Create a simple command
        let command = Command::new(0x80, 0xCA, 0x00, 0x00).with_le(0);

        // Process the command
        let result = channel.process_command(&command, &mut transport);

        // Verify command was processed successfully
        assert!(result.is_ok());

        // Verify security level
        assert_eq!(channel.security_level(), SecurityLevel::MACProtection);

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
    fn test_secure_channel_provider() {
        // Create mock transport
        let mut transport = TestMockTransport::new();

        // Create test session
        let session = create_test_session();

        // Create provider
        let provider = GPSecureChannelProvider::new(session);

        // Create secure channel
        let channel_result =
            provider.create_secure_channel(&mut transport, SecurityLevel::MACProtection);

        assert!(channel_result.is_ok());

        // Check the created channel has the correct security level
        let channel = channel_result.unwrap();
        assert_eq!(channel.security_level(), SecurityLevel::MACProtection);
        assert!(channel.is_active());
    }
}
