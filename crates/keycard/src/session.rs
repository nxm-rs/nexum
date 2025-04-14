//! Session management for the Keycard secure channel.
//!
//! This module provides the Session type that holds the session state
//! and derives session keys from the card keys and pairing information.

use bytes::Bytes;
use cipher::{Iv, Key};
use k256::{PublicKey, SecretKey};
use nexum_apdu_core::{
    ApduCommand, ApduResponse, CardTransport, processor::SecureProtocolError,
    response::error::ResponseError,
};
use rand_v8::thread_rng;
use zeroize::Zeroize;

use crate::{
    Challenge, OpenSecureChannelCommand, PairingInfo,
    crypto::{
        ApduMeta, KeycardScp, calculate_mac, derive_session_keys, generate_ecdh_shared_secret,
    },
};

/// Keycard SCP keys
#[derive(Debug, Clone, Zeroize)]
#[zeroize(drop)]
pub struct Keys {
    /// Encryption key
    enc: Key<KeycardScp>,
    /// MAC key
    mac: Key<KeycardScp>,
}

impl Keys {
    /// Create a new key set with the specified encryption and MAC keys.
    fn new(enc: Key<KeycardScp>, mac: Key<KeycardScp>) -> Self {
        Self { enc, mac }
    }

    /// Get the encryption key
    pub(crate) fn enc(&self) -> &Key<KeycardScp> {
        &self.enc
    }

    /// Get the MAC key
    pub(crate) fn mac(&self) -> &Key<KeycardScp> {
        &self.mac
    }
}

/// Session state for Keycard SCP secure channel
#[derive(Clone)]
pub struct Session {
    /// Session keys derived from the card keys and pairing information.
    keys: Keys,
    /// IV
    iv: Iv<KeycardScp>,
}

impl Session {
    pub fn new(
        card_public_key: &PublicKey,
        pairing_info: &PairingInfo,
        transport: &mut dyn CardTransport,
    ) -> Result<Self, SecureProtocolError> {
        // Generate an ephemeral keypair for the host for this session
        let host_private_key = SecretKey::random(&mut thread_rng());

        let cmd = OpenSecureChannelCommand::with_pairing_index_and_pubkey(
            pairing_info.index,
            &host_private_key.public_key(),
        );

        // Send the command
        let response_bytes = transport
            .transmit_raw(&cmd.to_command().to_bytes())
            .map_err(ResponseError::from)?;
        let response = nexum_apdu_core::Response::from_bytes(&response_bytes)?;

        // Check for errors
        if !response.is_success() {
            return Err(SecureProtocolError::Protocol("Open secure channel failed"));
        }

        match response.payload() {
            Some(payload) => {
                if payload.len() != 48 {
                    return Err(SecureProtocolError::Protocol(
                        "Invalid response data length",
                    ));
                }

                // Generate the shared secret
                let shared_secret = generate_ecdh_shared_secret(&host_private_key, card_public_key);

                // Derive the session keys
                let challenge = Challenge::from_slice(&payload[..32]);
                let iv = Iv::<KeycardScp>::clone_from_slice(&payload[32..48]);
                let (enc_key, mac_key) =
                    derive_session_keys(shared_secret, &pairing_info.key, challenge);

                Ok(Self {
                    keys: Keys::new(enc_key, mac_key),
                    iv,
                })
            }
            None => Err(SecureProtocolError::Protocol("No response payload")),
        }
    }

    pub fn from_raw(
        enc_key: &Key<KeycardScp>,
        mac_key: &Key<KeycardScp>,
        iv: &Iv<KeycardScp>,
    ) -> Self {
        Self {
            keys: Keys::new(*enc_key, *mac_key),
            iv: *iv,
        }
    }

    pub const fn keys(&self) -> &Keys {
        &self.keys
    }

    pub const fn iv(&self) -> &Iv<KeycardScp> {
        &self.iv
    }

    pub(crate) fn update_iv(&mut self, meta: &ApduMeta, data: &Bytes) {
        self.iv = calculate_mac(meta, data, self.keys.mac());
    }
}
