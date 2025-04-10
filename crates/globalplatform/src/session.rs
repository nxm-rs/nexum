//! Session management for SCP02 secure channel
//!
//! This module provides the Session type that holds the session state
//! and derives session keys from the card keys.

use cipher::Key;
use zeroize::Zeroize;

use crate::{
    Error, InitializeUpdateResponse, Result,
    constants::scp,
    crypto::{
        CardChallenge, DERIVATION_ENC, DERIVATION_MAC, HostChallenge, Scp02, SequenceCounter,
        calculate_cryptogram, derive_key,
    },
};

/// Secure Channel Protocol (SCP) keys
#[derive(Debug, Clone, Zeroize)]
#[zeroize(drop)]
pub struct Keys {
    /// Encryption key
    enc: Key<Scp02>,
    /// MAC key
    mac: Key<Scp02>,
}

impl Default for Keys {
    fn default() -> Self {
        // Default GlobalPlatform test key
        let key = [
            0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D,
            0x4E, 0x4F,
        ];
        let key = Key::<Scp02>::from_slice(&key);
        Self::from_single_key(*key)
    }
}

impl Keys {
    /// Create a new key set with the specified encryption and MAC keys
    pub const fn new(enc: Key<Scp02>, mac: Key<Scp02>) -> Self {
        Self { enc, mac }
    }

    /// Create a new key set where all keys are the same
    pub const fn from_single_key(key: Key<Scp02>) -> Self {
        Self { enc: key, mac: key }
    }

    /// Get the encryption key
    pub const fn enc(&self) -> &Key<Scp02> {
        &self.enc
    }

    /// Get the MAC key
    pub const fn mac(&self) -> &Key<Scp02> {
        &self.mac
    }
}

/// Session state for SCP02 secure channel
#[derive(Debug, Clone)]
pub struct Session {
    /// Session keys derived from card keys
    keys: Keys,
    /// Card challenge received during initialization
    card_challenge: CardChallenge,
    /// Host challenge sent during initialization
    host_challenge: HostChallenge,
    /// Sequence counter
    sequence_counter: SequenceCounter,
}

impl Session {
    /// Create a new session from an initialization response
    ///
    /// This validates the card's cryptogram and derives the session keys.
    ///
    /// # Arguments
    ///
    /// * `card_keys` - The keys shared with the card
    /// * `init_response` - The successful INITIALIZE UPDATE response
    /// * `host_challenge` - The challenge sent to the card
    ///
    /// # Returns
    ///
    /// A new Session if the card's cryptogram is valid
    pub fn from_response(
        keys: &Keys,
        init_response: &InitializeUpdateResponse,
        host_challenge: HostChallenge,
    ) -> Result<Self> {
        // Extract data from the response
        let (sequence_counter, card_challenge, card_cryptogram) = match init_response {
            InitializeUpdateResponse::Success {
                key_info,
                sequence_counter,
                card_challenge,
                card_cryptogram,
                ..
            } => {
                // Check SCP version
                let scp_version = key_info[1];
                if scp_version != scp::SCP02 {
                    return Err(Error::UnsupportedScpVersion(scp_version));
                }

                (sequence_counter, card_challenge, card_cryptogram)
            }
            _ => {
                return Err(Error::InvalidResponse(
                    "Not a successful INITIALIZE UPDATE response",
                ));
            }
        };

        // Derive session keys
        let session_enc = derive_key(keys.enc(), sequence_counter, &DERIVATION_ENC)?;
        let session_mac = derive_key(keys.mac(), sequence_counter, &DERIVATION_MAC)?;

        // Create session with the derived keys
        let keys = Keys::new(session_enc, session_mac);

        // Verify the card's cryptogram
        if *card_cryptogram
            != calculate_cryptogram(
                keys.enc(),
                sequence_counter,
                card_challenge,
                &host_challenge,
                false,
            )
        {
            return Err(Error::AuthenticationFailed("Invalid card cryptogram"));
        }

        Ok(Self {
            keys,
            card_challenge: *card_challenge,
            host_challenge,
            sequence_counter: *sequence_counter,
        })
    }

    // Keep the original method for backward compatibility but implement it in terms of from_response

    /// Get the session keys
    pub const fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Get the sequence counter
    pub const fn sequence_counter(&self) -> &SequenceCounter {
        &self.sequence_counter
    }

    /// Get the card challenge
    pub const fn card_challenge(&self) -> &CardChallenge {
        &self.card_challenge
    }

    /// Get the host challenge
    pub const fn host_challenge(&self) -> &HostChallenge {
        &self.host_challenge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_session_new() {
        // Test data based on actual card exchanges
        let card_key =
            Key::<Scp02>::from_slice(hex!("404142434445464748494a4b4c4d4e4f").as_slice());
        let keys = Keys::from_single_key(*card_key);
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000");
        let host_challenge = hex!("f0467f908e5ca23f");

        let response = InitializeUpdateResponse::from_bytes(&init_response).unwrap();
        let session = Session::from_response(&keys, &response, host_challenge);
        assert!(session.is_ok());

        // Verify extracted data
        let session = session.unwrap();
        assert_eq!(session.sequence_counter(), &[0x00, 0x0d]);
    }

    #[test]
    fn test_session_bad_response() {
        let key = Key::<Scp02>::default();
        let keys = Keys::from_single_key(key);
        let host_challenge = hex!("f0467f908e5ca23f");

        // Wrong SCP version
        let init_response = hex!("000002650183039536622001000de9c62ba1c4c8e55fcb91b6654ce49000");
        let response = InitializeUpdateResponse::from_bytes(&init_response).unwrap();
        let session = Session::from_response(&keys, &response, host_challenge);
        assert!(session.is_err());

        // Invalid cryptogram
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce40000");
        let response = InitializeUpdateResponse::from_bytes(&init_response).unwrap();
        let session = Session::from_response(&keys, &response, host_challenge);
        assert!(session.is_err());
    }

    #[test]
    fn test_keys_from_single_key() {
        let key = Key::<Scp02>::from_slice(hex!("404142434445464748494a4b4c4d4e4f").as_slice());
        let keys = Keys::from_single_key(*key);

        assert_eq!(keys.enc(), key);
        assert_eq!(keys.mac(), key);
    }
}
