//! Keycard application implementation
//!
//! This module provides the main Keycard application interface,
//! which encapsulates all the functionality for interacting with a Keycard.

use k256::ecdsa::RecoveryId;
use nexum_apdu_core::prelude::*;
use nexum_apdu_globalplatform::commands::select::SelectCommand;

use crate::constants::KEYCARD_AID;
use crate::secure_channel::{
    KeycardSecureChannel, KeycardSecureChannelExt, PairingProvider, PinProvider,
};
use crate::types::{Capabilities, Capability, ExportedKey, Signature, Version};
use crate::validation::{get_valid_pairing_index, get_valid_pairing_key, get_valid_pin};
use crate::{ApplicationInfo, ApplicationStatus, Error, PairingInfo, Result};
use crate::{Secrets, commands::*};
use alloy_primitives::hex;
use coins_bip32::path::DerivationPath;
use std::sync::Arc;
use tracing::{debug, warn};

/// Type for function that provides an input string (ie. PIN)
pub type InputRequestFn = Box<dyn Fn(&str) -> String + Send + Sync>;
/// Type for function that confirms operations
pub type ConfirmationFn = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// Keycard application implementation
pub struct Keycard<E: Executor> {
    /// Command executor
    executor: E,
    /// Pairing information for secure channel
    pairing_info: Option<PairingInfo>,
    /// Card public key (required for secure channel)
    card_public_key: Option<k256::PublicKey>,
    /// Application info retrieved during selection
    application_info: Option<ApplicationInfo>,
    /// Card capabilities
    capabilities: Capabilities,
    /// Optional callback for requesting input
    ///
    /// When None, operations requiring user input will fail
    input_request_callback: Option<InputRequestFn>,
    /// Optional callback for confirming critical operations
    ///
    /// When None, operations requiring confirmation will fail
    confirmation_callback: Option<ConfirmationFn>,
}

impl<E: Executor> Keycard<E> {
    /// Create a new Keycard from a CardTransport with callbacks for user interaction
    ///
    /// This constructor allows providing optional PIN and pairing information.
    /// If provided, it will set up the secure channel with the provided information.
    /// If not provided, it will use the callbacks to request information when needed.
    pub fn from_interactive<T>(
        mut transport: T,
        input_request_callback: InputRequestFn,
        confirmation_callback: ConfirmationFn,
        pin: Option<String>,
        pairing_info: Option<PairingInfo>,
    ) -> Result<Self>
    where
        T: CardTransport + 'static,
        E: From<CardExecutor<crate::secure_channel::KeycardSecureChannel<T>>>,
    {
        // First, select the Keycard application to get the card's info directly from the transport
        let app_info = select_keycard_with_transport(&mut transport)?;

        // Get the card's public key - this is required for secure channel
        let card_public_key = match app_info.public_key {
            Some(key) => key,
            None => {
                return Err(Error::Message(
                    "Card doesn't support secure channel: no public key available".to_string(),
                ));
            }
        };

        // Create a secure channel with the transport
        let mut secure_channel = KeycardSecureChannel::new(transport);

        // We need to use Arc to share the callbacks across closures
        let input_callback = Arc::new(input_request_callback);

        // Create PIN provider - either from provided PIN or create a callback with validation
        let pin_provider = match pin {
            Some(pin_str) => {
                // Validate the provided PIN
                match crate::validation::validate_pin(&pin_str) {
                    Ok(valid_pin) => PinProvider::Pin(valid_pin),
                    Err(e) => {
                        warn!("Provided PIN is invalid: {}", e);

                        // Fall back to callback method if the provided PIN is invalid
                        let callback_ref = Arc::clone(&input_callback);
                        let callback =
                            Box::new(move || get_valid_pin(&|prompt| callback_ref(prompt), 3));
                        PinProvider::Callback(callback)
                    }
                }
            }
            None => {
                // Clone Arc for the closure
                let callback_ref = Arc::clone(&input_callback);

                // Create a new callback for PIN that prompts the user with validation
                let callback = Box::new(move || get_valid_pin(&|prompt| callback_ref(prompt), 3));
                PinProvider::Callback(callback)
            }
        };

        // Create pairing provider - either from provided info or create a callback with validation
        let pairing_provider = match pairing_info.clone() {
            Some(info) => PairingProvider::Info(info),
            None => {
                // Clone Arc for the closure
                let callback_ref = Arc::clone(&input_callback);

                // Create a new callback for pairing with validation
                let callback = Box::new(move || {
                    // Get a valid pairing key (32 bytes) using the validation module
                    let key = get_valid_pairing_key(&|prompt| callback_ref(prompt), 3);

                    // Get a valid pairing index (0-99) using the validation module
                    let index = get_valid_pairing_index(&|prompt| callback_ref(prompt), 3);

                    debug!(
                        "Using pairing key and index: {} (index: {})",
                        hex::encode(key),
                        index
                    );

                    PairingInfo { key, index }
                });

                PairingProvider::Callback(callback)
            }
        };

        // Configure the secure channel with the providers
        secure_channel.configure_providers(card_public_key, pairing_provider, pin_provider);

        // Create an executor from the secure channel
        let executor = E::from(CardExecutor::new(secure_channel));

        // Extract the callback from the Arc for the Keycard instance
        let extracted_callback = Some(match Arc::try_unwrap(input_callback) {
            Ok(callback) => callback,
            // If there are still other references, create a new one that forwards to one of the clones
            Err(arc) => {
                let callback: InputRequestFn = Box::new(move |prompt: &str| arc(prompt));
                callback
            }
        });

        // Create the Keycard instance directly
        let keycard = Self {
            executor,
            pairing_info,
            card_public_key: Some(card_public_key),
            application_info: Some(app_info.clone()),
            capabilities: app_info.capabilities,
            input_request_callback: extracted_callback,
            confirmation_callback: Some(confirmation_callback),
        };

        Ok(keycard)
    }

    /// Create a new Keycard instance directly from a transport with callbacks for user interaction
    ///
    /// This constructor provides an easy way to create a Keycard instance without
    /// explicitly managing pairing information or PIN. It will prompt the user for
    /// these credentials when needed using the provided callbacks.
    pub fn new<T>(
        transport: T,
        input_request_callback: InputRequestFn,
        confirmation_callback: ConfirmationFn,
    ) -> Result<Self>
    where
        T: CardTransport + 'static,
        E: From<CardExecutor<crate::secure_channel::KeycardSecureChannel<T>>>,
    {
        // Create with interactive mode, but no pre-defined PIN or pairing info
        Self::from_interactive(
            transport,
            input_request_callback,
            confirmation_callback,
            None, // No PIN provided
            None, // No pairing info provided
        )
    }

    /// Create a new Keycard instance with provided pairing information
    ///
    /// This constructor automatically selects the Keycard application and configures
    /// the secure channel with the provided pairing information. It will still prompt
    /// for PIN when needed via the callbacks.
    pub fn with_pairing<T>(
        transport: T,
        pairing_info: PairingInfo,
        input_request_callback: InputRequestFn,
        confirmation_callback: ConfirmationFn,
    ) -> Result<Self>
    where
        T: CardTransport + 'static,
        E: From<CardExecutor<crate::secure_channel::KeycardSecureChannel<T>>>,
    {
        // Create with interactive mode, but with pre-defined pairing info
        Self::from_interactive(
            transport,
            input_request_callback,
            confirmation_callback,
            None, // No PIN provided
            Some(pairing_info),
        )
    }

    /// Create a Keycard instance optimized for programmatic use with known credentials
    ///
    /// This constructor is useful for automated signing processes where the PIN, pairing key,
    /// and index are known in advance. It creates a Keycard instance with no callbacks, suitable
    /// for non-interactive programmatic use.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use for communicating with the card
    /// * `pin` - The PIN to use for authentication
    /// * `pairing_info` - The pairing information including key and index
    pub fn with_known_credentials<T>(
        transport: T,
        pin: String,
        pairing_info: PairingInfo,
    ) -> Result<Self>
    where
        T: CardTransport + 'static,
        E: From<CardExecutor<crate::secure_channel::KeycardSecureChannel<T>>>,
    {
        // Validate PIN
        let _ = crate::validation::validate_pin(&pin)
            .map_err(|e| Error::Message(format!("Invalid PIN: {}", e)))?;

        // Create no-op callbacks that will panic if called
        // These should never be called since we have all credentials up front
        let input_callback: InputRequestFn = Box::new(|prompt| {
            panic!(
                "Input callback called when using known credentials: {}",
                prompt
            );
        });

        let confirm_callback: ConfirmationFn = Box::new(|prompt| {
            panic!(
                "Confirmation callback called when using known credentials: {}",
                prompt
            );
            #[allow(unreachable_code)]
            false
        });

        // Use from_interactive with known credentials
        Self::from_interactive(
            transport,
            input_callback,
            confirm_callback,
            Some(pin),
            Some(pairing_info),
        )
    }

    /// Get the pairing info for this Keycard
    pub fn pairing_info(&self) -> Option<&PairingInfo> {
        self.pairing_info.as_ref()
    }

    /// Select the Keycard application on the device using the default AID
    pub fn select_keycard(&mut self) -> Result<ApplicationInfo> {
        // Use the new standalone function with the executor's transport
        let app_info = select_keycard_with_transport(self.executor.transport_mut())?;

        // Store the application info
        self.application_info = Some(app_info.clone());

        // Store capabilities
        self.capabilities = app_info.capabilities;

        // Extract and store the public key if available
        if let Some(pk) = &app_info.public_key {
            self.card_public_key = Some(*pk);
        }

        Ok(app_info)
    }

    /// Initialize the Keycard card (factory reset)
    /// IMPORTANT: This will erase all data on the card
    pub fn initialize(&mut self, secrets: &Secrets, confirm: bool) -> Result<()> {
        // Check if the card supports credential management
        self.capabilities
            .require_capability(Capability::CredentialsManagement)?;

        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation(
                "Initialize the card? This will erase all data and cannot be undone.",
            )?
        {
            return Err(Error::UserCancelled);
        }

        // Check if we have the card's public key
        match self.card_public_key {
            Some(card_pubkey) => {
                // Create the initialization command
                let cmd = InitCommand::with_card_pubkey_and_secrets(card_pubkey, secrets);

                // Execute the command
                self.executor.execute(&cmd)?;

                // Clear out any existing pairing info since the card has been reset
                self.pairing_info = None;

                Ok(())
            }
            None => Err(Error::InvalidData(
                "Card public key is required for initialization",
            )),
        }
    }

    /// Request input using the input request callback
    ///
    /// Returns an error if no callback is set
    fn request_input(&self, prompt: &str) -> Result<String> {
        match &self.input_request_callback {
            Some(callback) => Ok(callback(prompt)),
            None => Err(Error::UserInteractionError(
                "No input request callback set for operation requiring user input".to_string(),
            )),
        }
    }

    /// Confirm a critical operation using the confirmation callback
    ///
    /// Returns false if no callback is set, preventing destructive operations
    fn confirm_operation(&self, operation_description: &str) -> Result<bool> {
        match &self.confirmation_callback {
            Some(callback) => Ok(callback(operation_description)),
            None => Err(Error::UserInteractionError(
                "No confirmation callback set for operation requiring confirmation".to_string(),
            )),
        }
    }
}

impl<E> Keycard<E>
where
    E: Executor + SecureChannelExecutor,
    E::Transport: crate::secure_channel::KeycardSecureChannelExt,
{
    /// Check if the secure channel is open
    pub fn is_secure_channel_open(&self) -> bool {
        self.executor.has_secure_channel()
    }

    /// Open the secure channel with the card
    pub fn open_secure_channel(&mut self) -> Result<()> {
        // Check if the card supports secure channel
        self.capabilities
            .require_capability(Capability::SecureChannel)?;

        // Check if we have pairing info
        if self.pairing_info.is_none() {
            return Err(Error::Message(
                "No pairing information available".to_string(),
            ));
        }

        // Check if we have card public key
        if self.card_public_key.is_none() {
            return Err(Error::Message("No card public key available".to_string()));
        }

        // Open the secure channel
        self.executor.open_secure_channel().map_err(Error::from)
    }

    /// Pair with the card
    pub fn pair(&mut self) -> Result<PairingInfo> {
        // Check if the card supports secure channel
        self.capabilities
            .require_capability(Capability::SecureChannel)?;

        // Get the pairing password from the user
        let password = self.request_input("Enter pairing password")?;

        // Get underlying transport to perform pairing
        let transport = self.executor.transport_mut();

        // Perform pairing with updated method signature
        let pairing_info = transport.pair(&password)?;

        // Store the pairing info
        self.pairing_info = Some(pairing_info.clone());

        Ok(pairing_info)
    }

    /// Get the status of the Keycard application
    pub fn get_status(&mut self) -> Result<ApplicationStatus> {
        // Create the get status command
        let cmd = GetStatusCommand::application();

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Extract status from response
        match response {
            GetStatusOk::ApplicationStatus { status } => Ok(status),
            _ => Err(Error::Message("Unexpected response type".to_string())),
        }
    }

    /// Get the current key path from the Keycard
    pub fn get_key_path(&mut self) -> Result<coins_bip32::path::DerivationPath> {
        // Create the get status command for key path
        let cmd = GetStatusCommand::key_path();

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Extract path from response
        match response {
            GetStatusOk::KeyPathStatus { path } => Ok(path),
            _ => Err(Error::Message("Unexpected response type".to_string())),
        }
    }

    /// Generate a new key in the card
    pub fn generate_key(&mut self, confirm: bool) -> Result<[u8; 32]> {
        // Check if the card supports key management
        self.capabilities
            .require_capability(Capability::KeyManagement)?;

        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation(
                "Generate a new keypair on the card? This will overwrite any existing key.",
            )?
        {
            return Err(Error::UserCancelled);
        }

        // Create the command
        let cmd = GenerateKeyCommand::create();

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Return the key UID from the response
        let GenerateKeyOk::Success { key_uid } = response;
        Ok(key_uid)
    }

    /// Export the current key without derivation
    ///
    /// Returns an `ExportedKey` enum which contains the key data in a format that matches
    /// the requested export option:
    /// - `ExportOption::PrivateAndPublic` → Returns `ExportedKey::Complete`
    /// - `ExportOption::PublicKeyOnly` → Returns `ExportedKey::PublicOnly`
    /// - `ExportOption::ExtendedPublicKey` → Returns `ExportedKey::Extended`
    pub fn export_key(&mut self, what: ExportOption, path: &DerivationPath) -> Result<ExportedKey> {
        // Create command to export current key
        let cmd = ExportKeyCommand::from_path(what, path);

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Extract the keypair from the response
        let ExportKeyOk::Success { keypair } = response;

        // Convert to appropriate ExportedKey type based on what was requested
        ExportedKey::try_from_keypair(keypair, what)
    }

    /// Sign data with the current key
    pub fn sign(
        &mut self,
        data: &[u8],
        path: &DerivationPath,
        confirm: bool,
    ) -> Result<alloy_primitives::Signature> {
        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation(&format!(
                "Sign data with key from path {}?",
                path.derivation_string()
            ))?
        {
            return Err(Error::UserCancelled);
        }

        // Create the sign command - requires a 32-byte hash
        if data.len() != 32 {
            return Err(Error::InvalidData("Data to sign must be exactly 32 bytes"));
        }

        let data_array: [u8; 32] = data
            .try_into()
            .map_err(|_| Error::InvalidData("Failed to convert data to 32-byte array"))?;

        // Create sign command
        let cmd = SignCommand::with(&data_array, path)?;

        // Execute the command
        let SignOk::Success { signature } = self.executor.execute_secure(&cmd)?;
        let recovery_id = RecoveryId::trial_recovery_from_prehash(
            &signature.public_key.into(),
            data,
            &signature.signature,
        )?;

        let signature: alloy_primitives::Signature = (*signature.signature, recovery_id).into();

        // Return the signature from the response
        Ok(signature)
    }

    /// Change a credential (PIN, PUK, or pairing secret)
    pub fn change_credential(
        &mut self,
        credential_type: CredentialType,
        new_value: &str,
        confirm: bool,
    ) -> Result<()> {
        // Check if the card supports credentials management
        self.capabilities
            .require_capability(Capability::CredentialsManagement)?;

        // Create a description for the confirmation
        let description = match credential_type {
            CredentialType::Pin => "Change the PIN?",
            CredentialType::Puk => "Change the PUK?",
            CredentialType::PairingSecret => "Change the pairing secret?",
        };

        // Confirm the operation if a confirmation function is provided
        if confirm && !self.confirm_operation(description)? {
            return Err(Error::UserCancelled);
        }

        // Create the change command based on credential type
        let cmd = match credential_type {
            CredentialType::Pin => ChangePinCommand::with_pin(new_value),
            CredentialType::Puk => ChangePinCommand::with_puk(new_value),
            CredentialType::PairingSecret => ChangePinCommand::with_pairing_secret(new_value),
        };

        // Execute the command
        self.executor.execute_secure(&cmd)?;

        Ok(())
    }

    /// Unblock the PIN using the PUK
    pub fn unblock_pin(&mut self, puk: &str, new_pin: &str, confirm: bool) -> Result<()> {
        // Check if the card supports credentials management
        self.capabilities
            .require_capability(Capability::CredentialsManagement)?;

        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation("Unblock the PIN? This will set a new PIN using the PUK.")?
        {
            return Err(Error::UserCancelled);
        }

        // Create the unblock PIN command
        let cmd = UnblockPinCommand::with_puk_and_new_pin(puk, new_pin);

        // Execute the command
        self.executor.execute_secure(&cmd)?;

        Ok(())
    }

    /// Factory reset the card
    pub fn factory_reset(&mut self, confirm: bool) -> Result<()> {
        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation("Factory reset the card? This will erase all data.")?
        {
            return Err(Error::UserCancelled);
        }

        // Create the factory reset command
        let cmd = FactoryResetCommand::reset();

        // Execute the command
        self.executor.execute(&cmd)?;

        Ok(())
    }

    /// Remove the current key from the card
    pub fn remove_key(&mut self, confirm: bool) -> Result<()> {
        // Check if the card supports key management
        self.capabilities
            .require_capability(Capability::KeyManagement)?;

        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self
                .confirm_operation("Remove the current key from the card? This cannot be undone.")?
        {
            return Err(Error::UserCancelled);
        }

        // Create the remove key command
        let cmd = RemoveKeyCommand::remove();

        // Execute the command
        self.executor.execute_secure(&cmd)?;

        Ok(())
    }

    /// Set the pinless path for the card
    pub fn set_pinless_path(
        &mut self,
        path: Option<&coins_bip32::path::DerivationPath>,
        confirm: bool,
    ) -> Result<()> {
        // Create description for confirmation
        let description = match path {
            Some(p) => format!("Set the pinless path to {:?}?", p),
            None => "Clear the pinless path?".to_string(),
        };

        // Confirm the operation if a confirmation function is provided
        if confirm && !self.confirm_operation(&description)? {
            return Err(Error::UserCancelled);
        }

        // Create the command
        let cmd = match path {
            Some(p) => SetPinlessPathCommand::with_path(p),
            None => {
                // Manually handle clearing the path by sending an empty path
                let empty_path = coins_bip32::path::DerivationPath::default();
                SetPinlessPathCommand::with_path(&empty_path)
            }
        };

        // Execute the command
        self.executor.execute_secure(&cmd)?;

        Ok(())
    }

    /// Generate a mnemonic phrase of the specified length
    pub fn generate_mnemonic(
        &mut self,
        words: u8,
    ) -> Result<coins_bip39::Mnemonic<coins_bip39::English>> {
        // Check if the card supports key management
        self.capabilities
            .require_capability(Capability::KeyManagement)?;

        // Create the generate mnemonic command
        let cmd = GenerateMnemonicCommand::with_words(words)?;

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Convert to mnemonic phrase
        response.to_phrase()
    }

    /// Identify the card by signing a challenge
    pub fn ident(&mut self, challenge: Option<&[u8; 32]>) -> Result<Signature> {
        // Create the ident command
        let cmd = match challenge {
            Some(c) => IdentCommand::with_challenge(c),
            None => IdentCommand::with_random_challenge(),
        };

        // Execute the command
        let response = self.executor.execute(&cmd)?;

        // Return the signature from the response
        let IdentOk::Success { signature } = response;
        Ok(signature)
    }

    /// Load a key into the card
    pub fn load_key(
        &mut self,
        public_key: Option<k256::PublicKey>,
        private_key: k256::SecretKey,
        confirm: bool,
    ) -> Result<[u8; 32]> {
        // Check if the card supports key management
        self.capabilities
            .require_capability(Capability::KeyManagement)?;

        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation(
                "Load a new key into the card? This will overwrite any existing key.",
            )?
        {
            return Err(Error::UserCancelled);
        }

        // Create the load key command
        let cmd = LoadKeyCommand::load_keypair(public_key, private_key)?;

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Return the key UID from the response
        let LoadKeyOk::Success { key_uid } = response;
        Ok(key_uid)
    }

    /// Load an extended key into the card
    pub fn load_extended_key(
        &mut self,
        public_key: Option<k256::PublicKey>,
        private_key: k256::SecretKey,
        chain_code: [u8; 32],
        confirm: bool,
    ) -> Result<[u8; 32]> {
        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation(
                "Load an extended key into the card? This will overwrite any existing key.",
            )?
        {
            return Err(Error::UserCancelled);
        }

        // Create the load key command
        let cmd = LoadKeyCommand::load_extended_keypair(public_key, private_key, chain_code)?;

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Return the key UID from the response
        let LoadKeyOk::Success { key_uid } = response;
        Ok(key_uid)
    }

    /// Load a BIP39 seed into the card
    pub fn load_seed(&mut self, seed: &[u8; 64], confirm: bool) -> Result<[u8; 32]> {
        // Confirm the operation if a confirmation function is provided
        if confirm
            && !self.confirm_operation(
                "Load a BIP39 seed into the card? This will overwrite any existing key.",
            )?
        {
            return Err(Error::UserCancelled);
        }

        // Create the load key command
        let cmd = LoadKeyCommand::load_bip39_seed(seed);

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Return the key UID from the response
        let LoadKeyOk::Success { key_uid } = response;
        Ok(key_uid)
    }

    /// Unpair the card from a specific pairing index
    pub fn unpair(&mut self, index: u8, confirm: bool) -> Result<()> {
        // Check if the card supports secure channel
        self.capabilities
            .require_capability(Capability::SecureChannel)?;

        // Confirm the operation if a confirmation function is provided
        if confirm && !self.confirm_operation(&format!("Unpair slot {} from the card?", index))? {
            return Err(Error::UserCancelled);
        }

        // Create the unpair command
        let cmd = UnpairCommand::with_index(index);

        // Execute the command
        self.executor.execute_secure(&cmd)?;

        // If we unpaired our own slot, clear the pairing info
        if let Some(pairing_info) = &self.pairing_info {
            if pairing_info.index == index {
                self.pairing_info = None;
            }
        }

        Ok(())
    }

    /// Store data in the card
    pub fn store_data(&mut self, record: PersistentRecord, data: &[u8]) -> Result<()> {
        // If the persistent record is NDEF, check if the card has this capability
        if record == PersistentRecord::Ndef {
            // Check if the card supports key management
            self.capabilities.require_capability(Capability::Ndef)?;
        }

        // Create the store data command
        let cmd = StoreDataCommand::put(record, data);

        // Execute the command
        self.executor.execute_secure(&cmd)?;

        Ok(())
    }

    /// Get data from the card
    pub fn get_data(&mut self, record: PersistentRecord) -> Result<Vec<u8>> {
        // If the persistent record is NDEF, check if the card has this capability
        if record == PersistentRecord::Ndef {
            // Check if the card supports key management
            self.capabilities.require_capability(Capability::Ndef)?;
        }

        // Create the get data command
        let cmd = GetDataCommand::get(record);

        // Execute the command
        let response = self.executor.execute_secure(&cmd)?;

        // Extract data from the response
        let GetDataOk::Success { data } = response;
        Ok(data)
    }
}

/// Select the Keycard applet using the provided transport and return its application information
pub fn select_keycard_with_transport<T: CardTransport>(
    transport: &mut T,
) -> Result<ApplicationInfo> {
    // Create a select command for Keycard using the default AID
    let cmd = SelectCommand::with_aid(KEYCARD_AID.to_vec());

    // Execute the command
    let command_bytes = cmd.to_command().to_bytes();
    let response_bytes = transport.transmit_raw(&command_bytes)?;

    // Parse the response using raw bytes
    let select_response =
        SelectCommand::parse_response_raw(bytes::Bytes::copy_from_slice(&response_bytes))
            .map_err(|e| Error::Message(e.to_string()))?;

    // Parse the response
    let parsed = ParsedSelectOk::try_from(select_response)?;

    // Return the appropriate ApplicationInfo based on the response
    match parsed {
        ParsedSelectOk::InitializedWithKey(info) => Ok(info),
        ParsedSelectOk::InitializedNoKey(info) => Ok(info),
        ParsedSelectOk::Uninitialized(maybe_key) => {
            // Create a minimal ApplicationInfo for the uninitialized card
            let capabilities = if maybe_key.is_some() {
                // For uninitialized cards with a public key, assume they support
                // secure channel and credentials management capabilities
                Capabilities::new(&[Capability::SecureChannel, Capability::CredentialsManagement])
            } else {
                // Uninitialized without a public key has at least credentials management capabilities
                Capabilities::new(&[Capability::CredentialsManagement])
            };

            let app_info = ApplicationInfo {
                instance_uid: [0; 16],                   // Empty instance UID
                public_key: maybe_key,                   // Use the public key if available
                version: Version { major: 0, minor: 0 }, // Set version to 0.0
                remaining_slots: 0,                      // No pairing slots yet
                key_uid: None,                           // No key UID yet
                capabilities,                            // Use capabilities set above
            };

            Ok(app_info)
        }
    }
}

/// Credential type for changing credentials
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialType {
    /// PIN (authentication credential)
    Pin,
    /// PUK (unblocking credential)
    Puk,
    /// Pairing secret (pairing credential)
    PairingSecret,
}
