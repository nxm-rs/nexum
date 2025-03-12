//! Session management for SCP02 secure channel
//!
//! This module provides the Session type that holds the session state
//! and derives session keys from the card keys.

use zeroize::Zeroize;

use crate::{
    Error, InitializeUpdateResponse, Result,
    constants::scp,
    crypto::{DERIVATION_PURPOSE_ENC, DERIVATION_PURPOSE_MAC, derive_key, verify_cryptogram},
};

/// Secure Channel Protocol (SCP) keys
#[derive(Debug, Clone, Zeroize)]
#[zeroize(drop)]
pub struct Keys {
    /// Encryption key
    enc: [u8; 16],
    /// MAC key
    mac: [u8; 16],
}

impl Keys {
    /// Create a new key set with the specified encryption and MAC keys
    pub fn new(enc: [u8; 16], mac: [u8; 16]) -> Self {
        Self { enc, mac }
    }

    /// Create a new key set where all keys are the same
    pub fn from_single_key(key: [u8; 16]) -> Self {
        Self { enc: key, mac: key }
    }

    /// Get the encryption key
    pub fn enc(&self) -> &[u8; 16] {
        &self.enc
    }

    /// Get the MAC key
    pub fn mac(&self) -> &[u8; 16] {
        &self.mac
    }
}

/// Session state for SCP02 secure channel
#[derive(Debug, Clone)]
pub struct Session {
    /// Session keys derived from card keys
    keys: Keys,
    /// Card challenge received during initialization
    card_challenge: [u8; 6],
    /// Host challenge sent during initialization
    host_challenge: [u8; 8],
    /// Sequence counter
    sequence_counter: [u8; 2],
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
        host_challenge: [u8; 8],
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
        let session_enc = derive_key(keys.enc(), sequence_counter, &DERIVATION_PURPOSE_ENC)?;
        let session_mac = derive_key(keys.mac(), sequence_counter, &DERIVATION_PURPOSE_MAC)?;

        // Create session with the derived keys
        let keys = Keys::new(session_enc, session_mac);

        // Verify the card's cryptogram
        let verified = verify_cryptogram(
            keys.enc(),
            &sequence_counter,
            card_challenge,
            &host_challenge,
            card_cryptogram,
        )?;

        if !verified {
            return Err(Error::AuthenticationFailed("Invalid card cryptogram"));
        }

        Ok(Session {
            keys,
            card_challenge: *card_challenge,
            host_challenge,
            sequence_counter: *sequence_counter,
        })
    }

    // Keep the original method for backward compatibility but implement it in terms of from_response
    pub fn new(card_keys: &Keys, init_response: &[u8], host_challenge: [u8; 8]) -> Result<Self> {
        // Parse the raw response bytes
        let response = match InitializeUpdateResponse::from_bytes(init_response) {
            Ok(resp) => resp,
            Err(_) => {
                return Err(Error::InvalidResponse(
                    "Failed to parse INITIALIZE UPDATE response",
                ));
            }
        };

        Self::from_response(card_keys, &response, host_challenge)
    }

    /// Get the session keys
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Get the sequence counter
    pub fn sequence_counter(&self) -> &[u8; 2] {
        &self.sequence_counter
    }

    /// Get the card challenge
    pub fn card_challenge(&self) -> &[u8; 6] {
        &self.card_challenge
    }

    /// Get the host challenge
    pub fn host_challenge(&self) -> &[u8; 8] {
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
        let card_key = Keys::from_single_key(hex!("404142434445464748494a4b4c4d4e4f"));
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce49000");
        let host_challenge = hex!("f0467f908e5ca23f");

        let session = Session::new(&card_key, &init_response, host_challenge);
        assert!(session.is_ok());

        // Verify extracted data
        let session = session.unwrap();
        assert_eq!(session.sequence_counter(), &[0x00, 0x0d]);
    }

    #[test]
    // TODO: Check these tests.
    fn test_session_bad_response() {
        let card_key = Keys::from_single_key([0u8; 16]);
        let host_challenge = hex!("f0467f908e5ca23f");

        // Too short response
        let init_response = hex!("01026982");
        let session = Session::new(&card_key, &init_response, host_challenge);
        assert!(session.is_err());

        // Wrong SCP version
        let init_response = hex!("000002650183039536622001000de9c62ba1c4c8e55fcb91b6654ce49000");
        let session = Session::new(&card_key, &init_response, host_challenge);
        assert!(session.is_err());

        // Invalid cryptogram
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce40000");
        let session = Session::new(&card_key, &init_response, host_challenge);
        assert!(session.is_err());
    }

    #[test]
    fn test_keys_from_single_key() {
        let key = hex!("404142434445464748494a4b4c4d4e4f");
        let keys = Keys::from_single_key(key);

        assert_eq!(keys.enc(), &key);
        assert_eq!(keys.mac(), &key);
    }
}
