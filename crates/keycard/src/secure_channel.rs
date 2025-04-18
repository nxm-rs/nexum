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

/// Extension trait for KeycardSecureChannel functionality (pairing)
pub trait KeycardSecureChannelExt: CardTransport {
    /// Pair with the card using a password
    fn pair(&mut self, password: &str) -> crate::Result<PairingInfo>;
}

impl<T: CardTransport> KeycardSecureChannelExt for KeycardSecureChannel<T> {
    fn pair(&mut self, password: &str) -> crate::Result<PairingInfo> {
        self.pair(password).map_err(crate::Error::from)
    }
}

/// Callback function type for requesting pairing information
pub type PairingCallback = Box<dyn Fn() -> PairingInfo + Send + Sync>;

/// Provider for pairing information
pub enum PairingProvider {
    /// Concrete pairing information
    Info(PairingInfo),
    /// Callback to request pairing information when needed
    Callback(PairingCallback),
}

impl PairingProvider {
    /// Get the pairing info from this provider
    pub fn info(&self) -> crate::Result<&PairingInfo> {
        match self {
            Self::Info(info) => Ok(info),
            Self::Callback(_) => Err(crate::Error::Message(
                "Cannot get info directly from callback".to_string(),
            )),
        }
    }
}

/// Callback function type for requesting PIN
pub type PinCallback = Box<dyn Fn() -> String + Send + Sync>;

/// Provider for PIN information
pub enum PinProvider {
    /// Concrete PIN string
    Pin(String),
    /// Callback to request PIN when needed
    Callback(PinCallback),
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
    /// Provider for pairing information
    pairing_provider: Option<PairingProvider>,
    /// Provider for PIN information
    pin_provider: Option<PinProvider>,
    /// Card public key for session initialization
    card_public_key: Option<k256::PublicKey>,
}

impl<T: CardTransport> fmt::Debug for KeycardSecureChannel<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeycardSecureChannel")
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
            pairing_provider: None,
            pin_provider: None,
            card_public_key: None,
        }
    }

    /// Create a new secure channel with authentication providers
    pub fn new_with_providers(
        transport: T,
        card_public_key: k256::PublicKey,
        pairing_provider: PairingProvider,
        pin_provider: PinProvider,
    ) -> Self {
        Self {
            transport,
            session: None,
            security_level: SecurityLevel::none(),
            established: false,
            pairing_provider: Some(pairing_provider),
            pin_provider: Some(pin_provider),
            card_public_key: Some(card_public_key),
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

    /// Initialize the session for this secure channel using existing pairing info.
    fn initialize_session(
        &mut self,
        card_public_key: Option<&k256::PublicKey>,
        pairing_info: Option<&PairingInfo>,
    ) -> crate::Result<()> {
        // Determine the card public key to use
        let card_key = match card_public_key {
            Some(key) => {
                // If a key is provided, store it for future use
                self.card_public_key = Some(*key);
                key
            }
            None => {
                // Otherwise use the stored key
                self.card_public_key.as_ref().ok_or_else(|| {
                    crate::Error::Message("Card public key not provided and not stored".to_string())
                })?
            }
        };

        // Determine pairing info to use
        let pairing = match pairing_info {
            Some(info) => info,
            None => {
                match &mut self.pairing_provider {
                    Some(PairingProvider::Info(info)) => info,
                    Some(PairingProvider::Callback(callback)) => {
                        // Call the callback to get the pairing info
                        let info = callback();
                        // Replace the callback with the obtained info to avoid calling it again
                        self.pairing_provider = Some(PairingProvider::Info(info.clone()));
                        match &self.pairing_provider {
                            Some(provider) => provider.info()?,
                            None => unreachable!(),
                        }
                    }
                    None => {
                        return Err(crate::Error::Message(
                            "No pairing information provided".to_string(),
                        ));
                    }
                }
            }
        };

        // Create a new session
        let session = Session::new(card_key, pairing, &mut self.transport)?;

        // Store the session
        self.session = Some(session);

        Ok(())
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
            Err(_) => {
                self.close()?;
                Err(crate::Error::MutualAuthenticationFailed)
            }
        }
    }

    /// Verify PIN using the provided PIN string or the callback if available
    fn verify_pin(&mut self, pin: &str) -> Result<bool, Error> {
        // Make sure that we're at least at enc mac level for PIN verification
        if !self.security_level.satisfies(&SecurityLevel::enc_mac()) {
            self.open()?;
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
            // Initialize session if we have the necessary providers
            if self.card_public_key.is_some() && self.pairing_provider.is_some() {
                self.initialize_session(None, None)
                    .map_err(|_| Error::other("Failed to initialize session"))?;
            } else {
                return Err(Error::other(
                    "Session not initialized and missing card public key or pairing provider",
                ));
            }
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
        self.reset()?;
        self.established = false;
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

        // Check if we're already at or above the required level
        if self.security_level.satisfies(&level) {
            return Ok(());
        }

        // If we need encryption/integrity and don't have them, we need to establish the channel
        if (level.encryption && !self.security_level.encryption)
            || (level.integrity && !self.security_level.integrity)
        {
            self.close()?;
            self.open()?;
        }

        // If we need authentication and don't have it, we need to verify the PIN
        if level.authentication && !self.security_level.authentication {
            // Verify PIN using the provider
            let pin = match &self.pin_provider {
                Some(PinProvider::Pin(pin)) => pin.clone(),
                Some(PinProvider::Callback(callback)) => callback(),
                None => {
                    return Err(Error::other(
                        "PIN required for authentication but no PIN provider available",
                    ));
                }
            };

            self.verify_pin(&pin)?;
        }

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
        self.session = None;
        self.security_level = SecurityLevel::none();

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
            pairing_provider: None,
            pin_provider: None,
            card_public_key: None,
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
