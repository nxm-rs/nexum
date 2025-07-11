//! Session management for the Keycard secure channel.
//!
//! This module provides the Session type that holds the session state
//! and derives session keys from the card keys and pairing information.

use bytes::Bytes;
use cipher::{Iv, Key};
use k256::{PublicKey, SecretKey};
use nexum_apdu_core::prelude::*;
use rand_v8::thread_rng;
use zeroize::Zeroize;

use crate::{
    OpenSecureChannelCommand, OpenSecureChannelOk, PairingInfo,
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
    ) -> Result<Self, Error> {
        // Generate an ephemeral keypair for the host for this session
        let host_private_key = SecretKey::random(&mut thread_rng());

        // Generate the shared secret
        let shared_secret = generate_ecdh_shared_secret(&host_private_key, card_public_key);

        let cmd = OpenSecureChannelCommand::with_pairing_index_and_pubkey(
            pairing_info.index,
            &host_private_key.public_key(),
        );

        // Send the command
        let command_bytes = cmd.to_command().to_bytes();
        let response_bytes = transport.transmit_raw(&command_bytes)?;
        let response =
            OpenSecureChannelCommand::parse_response_raw(Bytes::copy_from_slice(&response_bytes))
                .map_err(|e| Error::Message(e.to_string()))?;

        // Extract the challenge and IV using pattern matching for type safety
        let OpenSecureChannelOk::Success { challenge, iv } = response;

        // Derive the session keys
        let (enc_key, mac_key) =
            derive_session_keys(shared_secret, &pairing_info.key.into(), &challenge);

        Ok(Self {
            keys: Keys::new(enc_key, mac_key),
            iv,
        })
    }

    #[cfg(test)]
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
