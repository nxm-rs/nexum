use std::fmt;

use alloy_primitives::hex::{self, encode};
use bytes::{Bytes, BytesMut};
use k256::elliptic_curve::generic_array::GenericArray;
use nexum_apdu_core::prelude::*;
use rand::{RngCore, rng};
use sha2::{Digest, Sha256};
use tracing::{debug, trace, warn};

use crate::commands::mutually_authenticate::MutuallyAuthenticateCommand;
use crate::commands::pin::VerifyPinCommand;
use crate::crypto::{calculate_cryptogram, decrypt_data, encrypt_data, generate_pairing_token};
use crate::session::Session;
use crate::types::PairingInfo;
use crate::{Challenge, MutuallyAuthenticateOk, PairCommand, PairOk};

/// Extension trait for KeycardSecureChannel functionality
pub trait KeycardSecureChannelExt: CardTransport {
    /// Initialize a session with card public key and pairing info
    fn initialize_session(
        &mut self,
        card_public_key: &k256::PublicKey,
        pairing_info: &PairingInfo,
    ) -> crate::Result<()>;

    /// Pair with the card using a password
    fn pair(&mut self, password: &str) -> crate::Result<PairingInfo>;

    /// Verify PIN to gain secure access
    fn verify_pin(&mut self, pin: &str) -> crate::Result<bool>;
}

/// Implement the extension trait for KeycardSecureChannel
impl<T: CardTransport> KeycardSecureChannelExt for KeycardSecureChannel<T> {
    fn initialize_session(
        &mut self,
        card_public_key: &k256::PublicKey,
        pairing_info: &PairingInfo,
    ) -> crate::Result<()> {
        self.initialize_session(card_public_key, pairing_info)
            .map_err(crate::Error::from)
    }

    fn pair(&mut self, password: &str) -> crate::Result<PairingInfo> {
        self.pair(password).map_err(crate::Error::from)
    }

    fn verify_pin(&mut self, pin: &str) -> crate::Result<bool> {
        self.verify_pin(pin).map_err(crate::Error::from)
    }
}

/// Secure Channel Protocol implementation for Keycard
pub struct KeycardSecureChannel<T: CardTransport> {
    /// The underlying transport
    transport: T,
    /// Session containing keys and state (None if not established)
    session: Option<Session>,
    /// Security level of the secure channel
    security_level: SecurityLevel,
    /// Whether the secure channel is established
    established: bool,
}

impl<T: CardTransport> fmt::Debug for KeycardSecureChannel<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeycardSCP")
            .field("security_level", &self.security_level)
            .field("established", &self.established)
            .field("session_initialized", &self.session.is_some())
            .finish()
    }
}

impl<T: CardTransport> KeycardSecureChannel<T> {
    /// Create a new secure channel instance with just a transport
    /// The secure channel is not established until `open()` is called
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            session: None,
            security_level: SecurityLevel::none(),
            established: false,
        }
    }

    /// Initialize the session for this secure channel using existing pairing info and public key
    /// This prepares the session but does not establish the secure channel yet
    pub fn initialize_session(
        &mut self,
        card_public_key: &k256::PublicKey,
        pairing_info: &PairingInfo,
    ) -> crate::Result<()> {
        // Create a new session
        let session = Session::new(card_public_key, pairing_info, &mut self.transport)?;

        // Store the session
        self.session = Some(session);

        // Mutually authenticate
        match self.authenticate() {
            Ok(_) => Ok(()),
            Err(err) => {
                self.session = None;
                Err(err)
            }
        }
    }

    /// Pair the card and initialize the secure channel
    /// This is a complete process to pair with a card
    pub fn pair(&mut self, pairing_secret: &str) -> crate::Result<PairingInfo> {
        debug!("Starting pairing process with pairing password");

        // Determine the shared secret
        let shared_secret = generate_pairing_token(pairing_secret);

        // Generate a random challenge
        let mut challenge = Challenge::default();
        rng().fill_bytes(&mut challenge);

        // Create PAIR (first step) command
        let cmd = PairCommand::with_first_stage(&challenge);

        // Send the command through the transport
        let response_bytes = self.transport.transmit_raw(&cmd.to_command().to_bytes())?;
        match PairCommand::parse_response_raw(response_bytes) {
            Ok(PairOk::FirstStageSuccess {
                cryptogram: card_cryptogram,
                challenge: card_challenge,
            }) => {
                let expected_cryptogram = calculate_cryptogram(&shared_secret, &challenge);
                if card_cryptogram != expected_cryptogram {
                    return Err(crate::Error::PairingFailed);
                }

                let client_cryptogram = calculate_cryptogram(&shared_secret, &card_challenge);

                let cmd = PairCommand::with_final_stage(&client_cryptogram);

                // Send the command through the transport
                let response_bytes = self.transmit_raw(&cmd.to_command().to_bytes())?;
                match PairCommand::parse_response_raw(response_bytes) {
                    Ok(PairOk::FinalStageSuccess {
                        pairing_index,
                        salt,
                    }) => {
                        let key = {
                            let mut hasher = Sha256::new();
                            hasher.update(shared_secret);
                            hasher.update(salt);
                            hasher.finalize()
                        };

                        debug!("Pairing successful with index {}", pairing_index);

                        Ok(PairingInfo {
                            key: key.into(),
                            index: pairing_index,
                        })
                    }
                    _ => Err(crate::Error::invalid_data("Invalid response")),
                }
            }
            _ => Err(crate::Error::invalid_data("Invalid response")),
        }
    }

    /// Verify PIN using the PIN request callback if available
    pub fn verify_pin(&mut self, pin: &str) -> crate::Result<bool> {
        if !self.is_established() {
            return Err(Error::other("Secure channel not established").into());
        }

        // Create the command
        let cmd = VerifyPinCommand::with_pin(pin);

        // Execute the command directly using transmit_raw, similar to pair command
        let command_bytes = cmd.to_command().to_bytes();
        let response_bytes = self.transmit_raw(&command_bytes)?;

        // Parse the response
        VerifyPinCommand::parse_response_raw(Bytes::copy_from_slice(&response_bytes))
            .map_err(|e| Error::Message(e.to_string()))?;

        // At this point, it's guaranteed that the PIN was verified successfully
        self.security_level = SecurityLevel::full();

        Ok(true)
    }

    /// Encrypt APDU command data for the secure channel
    /// This method assumes the secure channel is established and session is initialized
    fn protect_command(&mut self, command: &[u8]) -> crate::Result<Bytes> {
        debug!(
            "KeycardSCP protect_command: starting with raw command: {}",
            hex::encode(command)
        );

        // Parse the command into a Command object
        let command = Command::from_bytes(command)?;
        let payload = command.data().unwrap_or(&[]);

        debug!(
            "KeycardSCP protect_command: parsed command CLA={:02X} INS={:02X} P1={:02X} P2={:02X} data={}",
            command.class(),
            command.instruction(),
            command.p1(),
            command.p2(),
            hex::encode(payload)
        );

        // Ensure session is available
        let session = self.session.as_mut().unwrap();

        // Encrypt the command data using the established session
        let mut data_to_encrypt = BytesMut::from(payload);
        let encrypted_data = encrypt_data(&mut data_to_encrypt, session.keys().enc(), session.iv());

        debug!(
            "KeycardSCP protect_command: encrypted data: {}",
            hex::encode(&encrypted_data)
        );

        // Prepare metadata for MAC calculation
        let mut meta = GenericArray::default();
        meta[0] = command.class();
        meta[1] = command.instruction();
        meta[2] = command.p1();
        meta[3] = command.p2();
        meta[4] = (encrypted_data.len() + 16) as u8; // Add MAC size
        debug!(
            "KeycardSCP protect_command: MAC metadata: {}",
            hex::encode(meta)
        );

        // Update session IV / calculate MAC
        session.update_iv(&meta, &encrypted_data);
        debug!(
            "KeycardSCP protect_command: updated IV/MAC: {}",
            hex::encode(session.iv())
        );

        // Combine MAC and encrypted data
        let mut data = BytesMut::with_capacity(16 + encrypted_data.len());
        data.extend(session.iv());
        data.extend(&encrypted_data);

        debug!(
            "KeycardSCP protect_command: final protected payload: {}",
            hex::encode(&data)
        );

        // Create the protected command
        let protected_command = command.with_data(data);
        let result = protected_command.to_bytes();
        debug!(
            "KeycardSCP protect_command: final protected command: {}",
            hex::encode(&result)
        );

        Ok(result)
    }

    /// Process response data from the secure channel
    /// This method assumes the secure channel is established and session is initialized
    fn process_response(&mut self, response: &[u8]) -> crate::Result<Bytes> {
        // Parse the response
        let response = Response::from_bytes(response)?;

        // For non-success responses, return as-is without decryption
        if !response.is_success() {
            return Ok(Bytes::copy_from_slice(response.to_bytes().as_ref()));
        }

        // Ensure session is available
        let session = self.session.as_mut().unwrap();

        match response.payload() {
            Some(payload) => {
                let response_data = payload.to_vec();

                // Need at least a MAC (16 bytes)
                if response_data.len() < 16 {
                    warn!(
                        "Response data too short for secure channel: {}",
                        response_data.len()
                    );
                    return Err(Error::BufferTooSmall)?;
                }

                // Split into MAC and encrypted data
                let (rmac, rdata) = response_data.split_at(16);
                let rdata = Bytes::from(rdata.to_vec());

                // Prepare metadata for MAC verification
                let mut metadata = GenericArray::default();
                metadata[0] = response_data.len() as u8;

                // Decrypt the data
                let mut data_to_decrypt = BytesMut::from(&rdata[..]);
                let decrypted_data =
                    decrypt_data(&mut data_to_decrypt, session.keys().enc(), session.iv())?;

                // Update IV for MAC verification
                session.update_iv(&metadata, &rdata);

                // Verify MAC
                if rmac != session.iv().as_slice() {
                    warn!("MAC verification failed for secure channel response");
                    return Err(Error::protocol("Invalid response MAC"))?;
                }

                trace!("Decrypted response: len={}", decrypted_data.len());

                Ok(decrypted_data)
            }
            None => {
                // No data in response, just return the status
                Ok(Bytes::copy_from_slice(response.to_bytes().as_ref()))
            }
        }
    }

    /// Perform mutual authentication to establish the secure channel
    fn authenticate(&mut self) -> crate::Result<()> {
        debug!("Starting mutual authentication process");

        // Generate a random challenge
        let mut challenge = Challenge::default();
        rng().fill_bytes(&mut challenge);

        // Create the command
        let cmd = MutuallyAuthenticateCommand::with_challenge(&challenge);

        // Send through transport
        let response_bytes = self.transmit_raw(&cmd.to_command().to_bytes())?;

        // Parse the response
        match MutuallyAuthenticateCommand::parse_response_raw(response_bytes) {
            Ok(response) => {
                // If we end up here, we can verify that we are using the same MAC key as the card
                // and therefore mutual authentication was successful
                let MutuallyAuthenticateOk::Success { cryptogram } = response;
                debug!(
                    response = %encode(cryptogram),
                    "Mutual authentication successful"
                );

                // Update state
                self.established = true;
                self.security_level = SecurityLevel::enc_mac();

                Ok(())
            }
            Err(_) => Err(crate::Error::MutualAuthenticationFailed),
        }
    }
}

impl<T: CardTransport> SecureChannel for KeycardSecureChannel<T> {
    type UnderlyingTransport = T;

    fn transport(&self) -> &Self::UnderlyingTransport {
        &self.transport
    }

    fn transport_mut(&mut self) -> &mut Self::UnderlyingTransport {
        &mut self.transport
    }

    fn open(&mut self) -> Result<(), Error> {
        if self.is_established() {
            return Ok(());
        }

        // Check if session has been initialized
        if self.session.is_none() {
            return Err(Error::other(
                "Session not initialized. Call initialize_session() first",
            ));
        }

        // Perform mutual authentication to establish the secure channel
        self.authenticate()
            .map_err(|_| Error::AuthenticationFailed("Mutual authentication failed"))
    }

    fn is_established(&self) -> bool {
        self.established
    }

    fn close(&mut self) -> Result<(), Error> {
        debug!("Closing Keycard secure channel");
        self.established = false;
        self.security_level = SecurityLevel::none();
        Ok(())
    }

    fn security_level(&self) -> SecurityLevel {
        trace!(
            "KeycardSCP::security_level() returning {:?}",
            self.security_level
        );
        self.security_level
    }

    fn upgrade(&mut self, level: SecurityLevel) -> Result<(), Error> {
        trace!(
            "KeycardSCP::upgrade called with current level={:?}, requested level={:?}",
            self.security_level, level
        );

        if !self.is_established() {
            return Err(Error::other("Secure channel not established"));
        }

        // Check if we're already at or above the required level
        if self.security_level.satisfies(&level) {
            return Ok(());
        }

        // For Keycard SCP, we only support upgrading to authentication through PIN verification
        // which is now handled through the PIN callback
        if level.authentication && !self.security_level.authentication {
            return Err(Error::other(
                "Authentication upgrade must be done with verify_pin",
            ));
        }

        // We already have encryption and integrity in KeycardSCP
        Ok(())
    }
}

impl<T: CardTransport> CardTransport for KeycardSecureChannel<T> {
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Error> {
        trace!(
            "KeycardSCP::transmit_raw called with security_level={:?}, established={}",
            self.security_level,
            self.is_established()
        );

        // Log the raw command bytes
        debug!("KeycardSCP raw command: {}", hex::encode(command));

        if self.session.is_some() {
            debug!("KeycardSCP: protecting command and processing response through secure channel");

            // Apply SCP protection - only when secure channel is established
            let protected = self
                .protect_command(command)
                .map_err(|e| Error::message(e.to_string()))?;

            // Log the protected command
            debug!("KeycardSCP protected command: {}", hex::encode(&protected));

            // Send the protected command through the underlying transport
            let response = self.transport.transmit_raw(&protected)?;

            // Log the protected response
            debug!("KeycardSCP protected response: {}", hex::encode(&response));

            // Process the response through the secure channel
            let result = self
                .process_response(&response)
                .map_err(|e| Error::message(e.to_string()))?;

            // Log the processed response
            debug!("KeycardSCP processed response: {}", hex::encode(&result));

            Ok(result)
        } else {
            // If channel not established, pass through to underlying transport directly
            debug!("KeycardSCP: passing command through to underlying transport");
            let response = self.transport.transmit_raw(command)?;

            // Log the raw response
            debug!("KeycardSCP raw response: {}", hex::encode(&response));

            Ok(response)
        }
    }

    fn reset(&mut self) -> Result<(), Error> {
        // Reset the underlying transport
        self.transport.reset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeycardScp;
    use alloy_primitives::hex;
    use cipher::{Iv, Key};

    #[test]
    fn test_protect_command() {
        // Set up the same keys and IV as in the Go test
        let enc_key =
            hex::decode("FDBCB1637597CF3F8F5E8263007D4E45F64C12D44066D4576EB1443D60AEF441")
                .unwrap();
        let mac_key =
            hex::decode("2FB70219E6635EE0958AB3F7A428BA87E8CD6E6F873A5725A55F25B102D0F1F7")
                .unwrap();
        let iv = hex::decode("627E64358FA9BDCDAD4442BD8006E0A5").unwrap();

        // Create a session with the test keys and IV
        let session = Session::from_raw(
            Key::<KeycardScp>::from_slice(&enc_key),
            Key::<KeycardScp>::from_slice(&mac_key),
            Iv::<KeycardScp>::from_slice(&iv),
        );

        // Mock transport that returns predefined responses
        #[derive(Debug)]
        struct MockTransport;
        impl CardTransport for MockTransport {
            fn transmit_raw(&mut self, _command: &[u8]) -> Result<Bytes, Error> {
                unimplemented!()
            }
            fn reset(&mut self) -> Result<(), Error> {
                unimplemented!()
            }
        }

        // Create secure channel with the session
        let mut scp = KeycardSecureChannel {
            transport: MockTransport,
            session: Some(session),
            security_level: SecurityLevel::enc_mac(),
            established: true,
        };

        // Create the same command as in the Go test
        let data = hex::decode("D545A5E95963B6BCED86A6AE826D34C5E06AC64A1217EFFA1415A96674A82500")
            .unwrap();
        let command = Command::new_with_data(0x80, 0x11, 0x00, 0x00, data).to_bytes();

        // Protect the command
        let protected = scp.protect_command(&command).unwrap();
        let protected_cmd = Command::from_bytes(&protected).unwrap();

        // Check the result matches the expected data
        let expected_data = hex::decode(
            "BA796BF8FAD1FD50407B87127B94F5023EF8903AE926EAD8A204F961B8A0EDAEE7CCCFE7F7F6380CE2C6F188E598E4468B7DEDD0E807C18CCBDA71A55F3E1F9A"
        ).unwrap();
        assert_eq!(protected_cmd.data().unwrap(), &expected_data);

        // Check the IV matches the expected IV
        let expected_iv = hex::decode("BA796BF8FAD1FD50407B87127B94F502").unwrap();
        assert_eq!(scp.session.as_ref().unwrap().iv().to_vec(), expected_iv);
    }
}
