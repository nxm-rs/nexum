//! Session management for SCP02 secure channel
//!
//! This module provides the Session type that holds the session state
//! and derives session keys from the card keys.

use zeroize::Zeroize;

use crate::{
    Error, Result,
    constants::scp,
    crypto::{DERIVATION_PURPOSE_ENC, DERIVATION_PURPOSE_MAC, derive_key, verify_cryptogram},
    util::check_length,
};

/// Secure Channel Protocol (SCP) keys
#[derive(Debug, Clone, Zeroize)]
#[zeroize(drop)]
pub struct Keys {
    /// Encryption key
    enc: [u8; 16],
    /// MAC key
    mac: [u8; 16],
    /// Data encryption key (optional)
    dek: Option<[u8; 16]>,
}

impl Keys {
    /// Create a new key set with the specified encryption and MAC keys
    pub fn new(enc: [u8; 16], mac: [u8; 16]) -> Self {
        Self {
            enc,
            mac,
            dek: None,
        }
    }

    /// Create a new key set with all three keys
    pub fn new_with_dek(enc: [u8; 16], mac: [u8; 16], dek: [u8; 16]) -> Self {
        Self {
            enc,
            mac,
            dek: Some(dek),
        }
    }

    /// Create a new key set where all keys are the same
    pub fn from_single_key(key: [u8; 16]) -> Self {
        Self {
            enc: key,
            mac: key,
            dek: Some(key),
        }
    }

    /// Get the encryption key
    pub fn enc(&self) -> &[u8] {
        &self.enc
    }

    /// Get the MAC key
    pub fn mac(&self) -> &[u8] {
        &self.mac
    }

    /// Get the data encryption key
    pub fn dek(&self) -> Option<&[u8]> {
        self.dek.as_ref().map(|key| key.as_slice())
    }
}

/// Session state for SCP02 secure channel
#[derive(Debug, Clone)]
pub struct Session {
    /// Session keys derived from card keys
    keys: Keys,
    /// Card challenge received during initialization
    card_challenge: [u8; 8],
    /// Host challenge sent during initialization
    host_challenge: [u8; 8],
    /// Sequence counter
    sequence_counter: [u8; 2],
    /// Security level
    security_level: u8,
}

impl Session {
    /// Create a new session from an initialization response
    ///
    /// This validates the card's cryptogram and derives the session keys.
    ///
    /// # Arguments
    ///
    /// * `card_keys` - The keys shared with the card
    /// * `init_response` - The response data from INITIALIZE UPDATE
    /// * `host_challenge` - The challenge sent to the card
    ///
    /// # Returns
    ///
    /// A new Session if the card's cryptogram is valid
    pub fn new(card_keys: &Keys, init_response: &[u8], host_challenge: &[u8]) -> Result<Self> {
        // Validate inputs
        check_length(host_challenge, 8)?;

        // Check for error status words
        if init_response.len() < 2 {
            return Err(Error::InvalidLength {
                expected: 28,
                actual: init_response.len(),
            });
        }

        // Basic response validation
        if init_response.len() < 28 {
            return Err(Error::InvalidLength {
                expected: 28,
                actual: init_response.len(),
            });
        }

        // Check SCP version
        let scp_major_version = init_response[11];
        if scp_major_version != scp::SCP02 {
            return Err(Error::UnsupportedScpVersion(scp_major_version));
        }

        // Extract data
        let security_level = init_response[10];

        let mut sequence_counter = [0u8; 2];
        sequence_counter.copy_from_slice(&init_response[12..14]);

        let mut card_challenge = [0u8; 8];
        card_challenge.copy_from_slice(&init_response[12..20]);

        let card_cryptogram = &init_response[20..28];

        // Derive session keys
        let session_enc = derive_key(card_keys.enc(), &sequence_counter, &DERIVATION_PURPOSE_ENC)?;
        let session_mac = derive_key(card_keys.mac(), &sequence_counter, &DERIVATION_PURPOSE_MAC)?;

        // Create session with the derived keys
        let session_keys = if let Some(dek) = card_keys.dek() {
            let session_dek = derive_key(
                dek,
                &sequence_counter,
                &crate::crypto::DERIVATION_PURPOSE_DEK,
            )?;
            Keys::new_with_dek(session_enc, session_mac, session_dek)
        } else {
            Keys::new(session_enc, session_mac)
        };

        // Verify the card's cryptogram
        let verified = verify_cryptogram(
            session_keys.enc(),
            host_challenge,
            &card_challenge,
            card_cryptogram,
        )?;

        if !verified {
            return Err(Error::AuthenticationFailed("Invalid card cryptogram"));
        }

        let mut host_challenge_array = [0u8; 8];
        host_challenge_array.copy_from_slice(host_challenge);

        Ok(Session {
            keys: session_keys,
            card_challenge,
            host_challenge: host_challenge_array,
            sequence_counter,
            security_level,
        })
    }

    /// Get the session keys
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Get the card challenge
    pub fn card_challenge(&self) -> &[u8] {
        &self.card_challenge
    }

    /// Get the host challenge
    pub fn host_challenge(&self) -> &[u8] {
        &self.host_challenge
    }

    /// Get the sequence counter
    pub fn sequence_counter(&self) -> &[u8] {
        &self.sequence_counter
    }

    /// Get the security level
    pub fn security_level(&self) -> u8 {
        self.security_level
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

        let session = Session::new(&card_key, &init_response, &host_challenge);
        assert!(session.is_ok());

        // Verify extracted data
        let session = session.unwrap();
        assert_eq!(session.security_level(), 0x36);
        assert_eq!(session.sequence_counter(), &[0x00, 0x0d]);
    }

    #[test]
    fn test_session_bad_response() {
        let card_key = Keys::from_single_key(hex!("404142434445464748494a4b4c4d4e4f"));
        let host_challenge = hex!("f0467f908e5ca23f");

        // Too short response
        let init_response = hex!("01026982");
        let session = Session::new(&card_key, &init_response, &host_challenge);
        assert!(session.is_err());

        // Wrong SCP version
        let init_response = hex!("000002650183039536622001000de9c62ba1c4c8e55fcb91b6654ce49000");
        let session = Session::new(&card_key, &init_response, &host_challenge);
        assert!(session.is_err());

        // Invalid cryptogram
        let init_response = hex!("000002650183039536622002000de9c62ba1c4c8e55fcb91b6654ce40000");
        let session = Session::new(&card_key, &init_response, &host_challenge);
        assert!(session.is_err());
    }

    #[test]
    fn test_keys_from_single_key() {
        let key = hex!("404142434445464748494a4b4c4d4e4f");
        let keys = Keys::from_single_key(key);

        assert_eq!(keys.enc(), &key);
        assert_eq!(keys.mac(), &key);

        // Convert fixed-size array to slice for comparison
        if let Some(dek) = keys.dek() {
            assert_eq!(dek, key.as_slice());
        } else {
            panic!("DEK should be present");
        }
    }
}
