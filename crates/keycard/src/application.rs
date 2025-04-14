use std::ops::Deref;
use std::str::FromStr;

use alloy_primitives::hex::ToHexExt;
use coins_bip32::path::DerivationPath;
use k256::ecdsa::RecoveryId;
/// Keycard application implementation
///
/// This module provides the main Keycard application interface, which
/// encapsulates all the functionality for managing Keycards.
use nexum_apdu_core::prelude::*;
use nexum_apdu_globalplatform::SelectCommand;
use tracing::debug;

use crate::commands::{
    GenerateKeyCommand, GetStatusOk, InitCommand, KeyPath, SignCommand, VerifyPinCommand,
};
use crate::error::{Error, Result};
use crate::secure_channel::{KeycardSCP, KeycardSecureChannelProvider};

use crate::types::{ApplicationInfo, PairingInfo};
use crate::{
    ApplicationStatus, GenerateKeyOk, InitOk, KEYCARD_AID, ParsedSelectOk, Secrets, SignOk,
};

/// Keycard card management application
#[derive(Debug)]
pub struct Keycard<E>
where
    E: Executor + SecureChannelExecutor,
{
    /// Card executor
    executor: E,
    /// Secure channel provider - optional to support unpaired states
    secure_channel_provider: Option<KeycardSecureChannelProvider>,
    /// Application info from card selection
    application_info: Option<ApplicationInfo>,
}

impl<E> Keycard<E>
where
    E: Executor + SecureChannelExecutor,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    /// Create a new Keycard instance
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            secure_channel_provider: None,
            application_info: None,
        }
    }

    /// Create a new Keycard instance with existing pairing information
    pub fn with_pairing(
        executor: E,
        pairing_info: PairingInfo,
        card_public_key: k256::PublicKey,
    ) -> Self {
        let provider = KeycardSecureChannelProvider::new(pairing_info, card_public_key);

        Self {
            executor,
            secure_channel_provider: Some(provider),
            application_info: None,
        }
    }

    /// Select Keycard
    pub fn select_keycard(&mut self) -> Result<ParsedSelectOk> {
        self.select_application(KEYCARD_AID)
    }

    /// Select the application by AID
    pub fn select_application(&mut self, aid: &[u8]) -> Result<ParsedSelectOk> {
        // Create SELECT command
        debug!("Selecting application: {:?}", aid);
        let cmd = SelectCommand::with_aid(aid.to_vec());
        let result = self.executor.execute(&cmd)?;

        let app_select_response = ParsedSelectOk::try_from(result).map_err(|_| {
            Error::Response(nexum_apdu_core::response::error::ResponseError::Parse(
                "Unable to parse response",
            ))
        })?;
        if let ParsedSelectOk::ApplicationInfo(application_info) = &app_select_response {
            self.application_info = Some(application_info.clone());
        }

        Ok(app_select_response)
    }

    /// Initialize the keycard
    pub fn initialize(&mut self, secrets: &Secrets) -> Result<InitOk> {
        // First select the card to get into proper state
        let select_response = self.select_keycard()?;

        // Initialize the card
        self.init(select_response, secrets)
    }

    /// Init the keycard (internal implementation)
    fn init(&mut self, select_response: ParsedSelectOk, secrets: &Secrets) -> Result<InitOk> {
        // Create INIT command
        match select_response {
            ParsedSelectOk::PreInitialized(pre) => {
                let cmd = InitCommand::with_card_pubkey_and_secrets(
                    pre.ok_or(Error::SecureChannelNotSupported)?,
                    secrets,
                );

                Ok(self.executor.execute(&cmd)?)
            }
            _ => Err(Error::AlreadyInitialised),
        }
    }

    /// Pair with the card
    pub fn pair<F>(&mut self, pairing_pass: F) -> Result<PairingInfo>
    where
        F: FnOnce() -> String,
    {
        let pairing_info = KeycardSCP::pair(&mut self.executor, pairing_pass)?;

        // Store pairing info for future secure channel establishment
        if let Some(app_info) = &self.application_info {
            if let Some(public_key) = &app_info.public_key {
                self.secure_channel_provider = Some(KeycardSecureChannelProvider::new(
                    pairing_info.clone(),
                    *public_key,
                ));
            }
        }

        Ok(pairing_info)
    }

    /// Open secure channel using current pairing information
    pub fn open_secure_channel(&mut self) -> Result<()> {
        if self.secure_channel_provider.is_none() {
            return Err(Error::SecureProtocol(
                nexum_apdu_core::processor::SecureProtocolError::Other(
                    "No pairing information provided".to_string(),
                ),
            ));
        }

        let provider = self.secure_channel_provider.as_ref().unwrap();
        self.executor.open_secure_channel(provider)?;

        Ok(())
    }

    /// Get application status
    pub fn get_status(&mut self) -> Result<ApplicationStatus> {
        // Use typed GetStatusCommand instead of raw transmit
        use crate::commands::get_status::GetStatusCommand;

        let cmd = GetStatusCommand::application();
        let response = self.executor.execute(&cmd)?;

        match response {
            GetStatusOk::ApplicationStatus { status } => Ok(status),
            _ => unreachable!("Requested application status, should be unreachable"),
        }
    }

    /// Verify PIN
    pub fn verify_pin<F>(&mut self, pin: F) -> Result<()>
    where
        F: FnOnce() -> String,
    {
        // Create and execute the command
        // The command's required_security_level will be automatically enforced by executor
        let cmd = VerifyPinCommand::with_pin(&pin());
        let _ = self.executor.execute(&cmd)?;

        Ok(())
    }

    /// Generate a new key on the card
    pub fn generate_key(&mut self) -> Result<[u8; 32]> {
        // Create the command
        let cmd = GenerateKeyCommand::create();

        // Execute it (security requirements handled automatically by executor)
        let response = self.executor.execute(&cmd)?;

        let GenerateKeyOk::Success { key_uid } = response;
        Ok(key_uid)
    }

    /// Sign data with the key on the card
    pub fn sign(&mut self, data: &[u8; 32], path: &KeyPath) -> Result<alloy_primitives::Signature> {
        // Create sign command with path and data
        let cmd = SignCommand::with(data, path, None)?;

        // Execute the command
        let response = self.executor.execute(&cmd)?;

        let SignOk::Success { signature } = response;

        let recovery_id = RecoveryId::trial_recovery_from_prehash(
            &signature.public_key.into(),
            data,
            signature.signature.deref(),
        )?;

        let address = alloy_primitives::Address::from_public_key(&signature.public_key.into());
        let signature: alloy_primitives::Signature =
            (*signature.signature.deref(), recovery_id).into();

        println!("Recovery ID: {:?}", recovery_id);

        println!("Signing address: {:?}", address.encode_hex_with_prefix());
        println!(
            "Signature: {:?}",
            signature.as_bytes().encode_hex_with_prefix()
        );

        let recovered_address = signature.recover_address_from_prehash(data.into()).unwrap();
        println!(
            "Recovered address: {:?}",
            recovered_address.encode_hex_with_prefix()
        );
        Ok(signature)
    }

    /// Get the executor
    pub fn executor(&self) -> &E {
        &self.executor
    }

    /// Get mutable access to the executor
    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    /// Check if PIN is verified
    pub fn is_pin_verified(&self) -> bool {
        // Implementation depends on your internal security state tracking
        self.security_level().is_authenticated()
    }

    /// Check if secure channel is open
    pub fn is_secure_channel_open(&self) -> bool {
        self.executor.has_secure_channel()
    }

    /// Set or update pairing information
    pub fn set_pairing_info(&mut self, pairing_info: PairingInfo) {
        // Implementation depends on the actual state storage
        // This is just an example:
        if let Some(app_info) = &self.application_info {
            if let Some(public_key) = app_info.public_key {
                self.secure_channel_provider =
                    Some(KeycardSecureChannelProvider::new(pairing_info, public_key));
            }
        }
    }

    /// Get current pairing information
    pub fn pairing_info(&self) -> Option<&PairingInfo> {
        self.secure_channel_provider
            .as_ref()
            .map(|provider| provider.pairing_info())
    }

    /// Change credential (PIN, PUK, or pairing secret)
    pub fn change_credential<S>(
        &mut self,
        credential_type: CredentialType,
        new_value: S,
    ) -> Result<()>
    where
        S: AsRef<str>,
    {
        use crate::commands::ChangePinCommand;

        let cmd = match credential_type {
            CredentialType::Pin => ChangePinCommand::with_pin(new_value.as_ref()),
            CredentialType::Puk => ChangePinCommand::with_puk(new_value.as_ref()),
            CredentialType::PairingSecret => {
                ChangePinCommand::with_pairing_secret(new_value.as_ref().as_bytes())
            }
        };

        self.executor.execute(&cmd)?;
        Ok(())
    }

    /// Unblock PIN using PUK
    pub fn unblock_pin<S1, S2>(&mut self, puk: S1, new_pin: S2) -> Result<()>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        use crate::commands::UnblockPinCommand;

        let cmd = UnblockPinCommand::with_puk_and_new_pin(puk.as_ref(), new_pin.as_ref());
        self.executor.execute(&cmd)?;
        Ok(())
    }

    /// Remove the current key from the card
    pub fn remove_key(&mut self) -> Result<()> {
        use crate::commands::RemoveKeyCommand;

        let cmd = RemoveKeyCommand::remove();
        self.executor.execute(&cmd)?;
        Ok(())
    }

    /// Get security level for authentication status
    fn security_level(&self) -> SecurityLevel {
        // You might want to track this internally or ask the executor
        self.executor.security_level()
    }

    /// Change PIN
    pub fn change_pin(&mut self, new_pin: &str) -> Result<()> {
        use crate::commands::ChangePinCommand;
        let cmd = ChangePinCommand::with_pin(new_pin);
        self.executor.execute(&cmd)?;
        Ok(())
    }

    /// Change PUK
    pub fn change_puk(&mut self, new_puk: &str) -> Result<()> {
        use crate::commands::ChangePinCommand;
        let cmd = ChangePinCommand::with_puk(new_puk);
        self.executor.execute(&cmd)?;
        Ok(())
    }

    /// Change pairing secret
    pub fn change_pairing_secret(&mut self, new_secret: &[u8]) -> Result<()> {
        use crate::commands::ChangePinCommand;
        let cmd = ChangePinCommand::with_pairing_secret(new_secret);
        self.executor.execute(&cmd)?;
        Ok(())
    }

    /// Set a PIN-less path for signature operations
    pub fn set_pinless_path(&mut self, path: &str) -> Result<()> {
        use crate::commands::SetPinlessPathCommand;

        // Parse the path string into a DerivationPath
        let derivation_path = DerivationPath::from_str(path)?;

        let cmd = SetPinlessPathCommand::with_path(&derivation_path);
        self.executor.execute(&cmd)?;
        Ok(())
    }
}

/// Enum for credential types that can be changed
pub enum CredentialType {
    Pin,
    Puk,
    PairingSecret,
}
