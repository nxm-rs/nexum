use base64::prelude::*;
use bytes::{BufMut, Bytes, BytesMut};
use rand::{Rng, RngCore};

use crate::{
    AppletVersion,
    crypto::{PairingToken, generate_pairing_token},
};

const MAX_PUK_NUMBER: u64 = 999_999_999_999;
const MAX_PIN_NUMBER: u64 = 999_999;
const DEFAULT_MAX_PIN_ATTEMPTS: u8 = 3;
const DEFAULT_MAX_PUK_ATTEMPTS: u8 = 5;
const PIN_LENGTH: usize = 6;
const PUK_LENGTH: usize = 12;

/// Contains the secret data needed to pair a client with a card.
#[derive(Debug, Clone)]
pub struct Secrets {
    pin: String,
    puk: String,
    pairing_pass: String,
    pairing_token: PairingToken,
    version: AppletVersion,
    max_pin_attempts: u8,
    max_puk_attempts: u8,
    duress_pin: Option<String>,
}

impl Secrets {
    /// Creates a new Secrets instance with provided PIN, PUK and pairing password for legacy versions
    pub fn new(pin: &str, puk: &str, pairing_pass: &str) -> Self {
        // Validate input lengths
        assert_eq!(
            pin.len(),
            PIN_LENGTH,
            "PIN must be exactly {} digits",
            PIN_LENGTH
        );
        assert_eq!(
            puk.len(),
            PUK_LENGTH,
            "PUK must be exactly {} digits",
            PUK_LENGTH
        );

        Self {
            pin: pin.to_string(),
            puk: puk.to_string(),
            pairing_pass: pairing_pass.to_string(),
            pairing_token: generate_pairing_token(pairing_pass),
            version: AppletVersion::Legacy,
            max_pin_attempts: DEFAULT_MAX_PIN_ATTEMPTS,
            max_puk_attempts: DEFAULT_MAX_PUK_ATTEMPTS,
            duress_pin: None,
        }
    }

    /// Creates a new Secrets instance with provided PIN, PUK, pairing password,
    /// max attempts and optional duress PIN for version 3.1+
    pub fn new_v3_1(
        pin: &str,
        puk: &str,
        pairing_pass: &str,
        max_pin_attempts: u8,
        max_puk_attempts: u8,
        duress_pin: Option<String>,
    ) -> Self {
        // Validate input lengths
        assert_eq!(
            pin.len(),
            PIN_LENGTH,
            "PIN must be exactly {} digits",
            PIN_LENGTH
        );
        assert_eq!(
            puk.len(),
            PUK_LENGTH,
            "PUK must be exactly {} digits",
            PUK_LENGTH
        );

        // Validate duress PIN if provided
        if let Some(duress) = &duress_pin {
            assert_eq!(
                duress.len(),
                PIN_LENGTH,
                "Duress PIN must be exactly {} digits",
                PIN_LENGTH
            );
        }

        Self {
            pin: pin.to_string(),
            puk: puk.to_string(),
            pairing_pass: pairing_pass.to_string(),
            pairing_token: generate_pairing_token(pairing_pass),
            version: AppletVersion::V3_1,
            max_pin_attempts,
            max_puk_attempts,
            duress_pin,
        }
    }

    /// Generates a new Secrets with random PUK and pairing password for legacy versions
    pub fn generate() -> Self {
        let pairing_pass = generate_pairing_pass();

        let mut rng = rand::rng();
        let puk = rng.random_range(0..MAX_PUK_NUMBER);
        let pin = rng.random_range(0..MAX_PIN_NUMBER);

        Self {
            pin: format!("{:06}", pin),  // Ensure 6 digits with zero padding
            puk: format!("{:012}", puk), // Ensure 12 digits with zero padding
            pairing_pass: pairing_pass.clone(),
            pairing_token: generate_pairing_token(&pairing_pass),
            version: AppletVersion::Legacy,
            max_pin_attempts: DEFAULT_MAX_PIN_ATTEMPTS,
            max_puk_attempts: DEFAULT_MAX_PUK_ATTEMPTS,
            duress_pin: None,
        }
    }

    /// Generates a new Secrets with random PIN, PUK and pairing password for version 3.1+
    pub fn generate_v3_1(
        max_pin_attempts: u8,
        max_puk_attempts: u8,
        with_duress_pin: bool,
    ) -> Self {
        let pairing_pass = generate_pairing_pass();

        let mut rng = rand::rng();
        let puk = rng.random_range(0..MAX_PUK_NUMBER);
        let pin = rng.random_range(0..MAX_PIN_NUMBER);
        let puk_str = format!("{:012}", puk); // Ensure 12 digits with zero padding
        let pin_str = format!("{:06}", pin); // Ensure 6 digits with zero padding

        // Generate duress PIN if requested
        let duress_pin = if with_duress_pin {
            // Generate a different PIN for duress
            let duress = rng.random_range(0..MAX_PIN_NUMBER);
            Some(format!("{:06}", duress)) // Ensure 6 digits with zero padding
        } else {
            None
        };

        Self {
            pin: pin_str,
            puk: puk_str,
            pairing_pass: pairing_pass.clone(),
            pairing_token: generate_pairing_token(&pairing_pass),
            version: AppletVersion::V3_1,
            max_pin_attempts,
            max_puk_attempts,
            duress_pin,
        }
    }

    /// Returns the PIN string
    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// Returns the PUK string
    pub fn puk(&self) -> &str {
        &self.puk
    }

    /// Returns the pairing password string
    pub fn pairing_pass(&self) -> &str {
        &self.pairing_pass
    }

    /// Returns the pairing token generated from the random pairing password
    pub fn pairing_token(&self) -> &PairingToken {
        &self.pairing_token
    }

    /// Returns the max PIN attempts
    pub fn max_pin_attempts(&self) -> u8 {
        self.max_pin_attempts
    }

    /// Returns the max PUK attempts
    pub fn max_puk_attempts(&self) -> u8 {
        self.max_puk_attempts
    }

    /// Returns the duress PIN if set
    pub fn duress_pin(&self) -> Option<&str> {
        self.duress_pin.as_deref()
    }

    /// Returns the version of the applet these secrets are for
    pub fn version(&self) -> AppletVersion {
        self.version
    }

    /// Encodes the secrets to a Bytes object according to the applet version
    pub fn to_bytes(&self) -> Bytes {
        // Calculate the required capacity
        let capacity = match self.version {
            AppletVersion::Legacy => PIN_LENGTH + PUK_LENGTH + std::mem::size_of::<PairingToken>(),
            AppletVersion::V3_1 => {
                PIN_LENGTH + PUK_LENGTH + std::mem::size_of::<PairingToken>() + 1 + 1 + PIN_LENGTH
            }
        };

        let mut buffer = BytesMut::with_capacity(capacity);

        // Add PIN (6 bytes)
        debug_assert_eq!(
            self.pin.len(),
            PIN_LENGTH,
            "PIN must be exactly {} digits",
            PIN_LENGTH
        );
        buffer.put_slice(self.pin.as_bytes());

        // Add PUK (12 bytes)
        debug_assert_eq!(
            self.puk.len(),
            PUK_LENGTH,
            "PUK must be exactly {} digits",
            PUK_LENGTH
        );
        buffer.put_slice(self.puk.as_bytes());

        // Add pairing secret (32 bytes)
        debug_assert_eq!(
            self.pairing_token.len(),
            std::mem::size_of::<PairingToken>(),
            "Pairing token must be exactly {} bytes",
            std::mem::size_of::<PairingToken>()
        );
        buffer.put_slice(&self.pairing_token);

        // For version 3.1+, add max PIN attempts, max PUK attempts, and duress PIN
        if self.version == AppletVersion::V3_1 {
            // Add max PIN attempts (1 byte)
            buffer.put_u8(self.max_pin_attempts);

            // Add max PUK attempts (1 byte)
            buffer.put_u8(self.max_puk_attempts);

            // Add duress PIN (6 bytes) or default to first half of PUK if not provided
            if let Some(duress) = &self.duress_pin {
                debug_assert_eq!(
                    duress.len(),
                    PIN_LENGTH,
                    "Duress PIN must be exactly {} digits",
                    PIN_LENGTH
                );
                buffer.put_slice(duress.as_bytes());
            } else {
                // Use first half of PUK as default duress PIN
                buffer.put_slice(&self.puk.as_bytes()[0..PIN_LENGTH]);
            }
        }

        // Final verification of buffer length
        debug_assert_eq!(buffer.len(), capacity, "Buffer length mismatch");

        buffer.freeze()
    }
}

fn generate_pairing_pass() -> String {
    let mut r = vec![0u8; 12];
    rand::rng().fill_bytes(&mut r);
    BASE64_URL_SAFE_NO_PAD.encode(&r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secrets_new() {
        let secrets = Secrets::new("123456", "123456789012", "test-pairing-pass");
        assert_eq!(secrets.pin(), "123456");
        assert_eq!(secrets.puk(), "123456789012");
        assert_eq!(secrets.pairing_pass(), "test-pairing-pass");
        assert!(!secrets.pairing_token().is_empty());
        assert_eq!(secrets.version(), AppletVersion::Legacy);
    }

    #[test]
    #[should_panic(expected = "PIN must be exactly 6 digits")]
    fn test_invalid_pin_length() {
        Secrets::new("12345", "123456789012", "test-pairing-pass");
    }

    #[test]
    #[should_panic(expected = "PUK must be exactly 12 digits")]
    fn test_invalid_puk_length() {
        Secrets::new("123456", "12345678901", "test-pairing-pass");
    }

    #[test]
    #[should_panic(expected = "Duress PIN must be exactly 6 digits")]
    fn test_invalid_duress_pin_length() {
        Secrets::new_v3_1(
            "123456",
            "123456789012",
            "test-pairing-pass",
            5,
            7,
            Some("1234".to_string()),
        );
    }

    #[test]
    fn test_secrets_new_v3_1() {
        let secrets = Secrets::new_v3_1(
            "123456",
            "123456789012",
            "test-pairing-pass",
            5,
            7,
            Some("654321".to_string()),
        );
        assert_eq!(secrets.pin(), "123456");
        assert_eq!(secrets.puk(), "123456789012");
        assert_eq!(secrets.pairing_pass(), "test-pairing-pass");
        assert!(!secrets.pairing_token().is_empty());
        assert_eq!(secrets.version(), AppletVersion::V3_1);
        assert_eq!(secrets.max_pin_attempts(), 5);
        assert_eq!(secrets.max_puk_attempts(), 7);
        assert_eq!(secrets.duress_pin(), Some("654321"));
    }

    #[test]
    fn test_secrets_generate() {
        let secrets = Secrets::generate();

        // Check PIN format
        assert_eq!(secrets.pin().len(), PIN_LENGTH);
        assert!(secrets.pin().parse::<u64>().is_ok());

        // Check PUK format
        assert_eq!(secrets.puk().len(), PUK_LENGTH);
        assert!(secrets.puk().parse::<u64>().is_ok());

        // Check pairing pass and token
        assert!(!secrets.pairing_pass().is_empty());
        assert_eq!(
            secrets.pairing_token().len(),
            std::mem::size_of::<PairingToken>()
        );

        // Check version
        assert_eq!(secrets.version(), AppletVersion::Legacy);
    }

    #[test]
    fn test_secrets_generate_v3_1() {
        let secrets = Secrets::generate_v3_1(4, 6, true);

        // Check PIN format
        assert_eq!(secrets.pin().len(), PIN_LENGTH);
        assert!(secrets.pin().parse::<u64>().is_ok());

        // Check PUK format
        assert_eq!(secrets.puk().len(), PUK_LENGTH);
        assert!(secrets.puk().parse::<u64>().is_ok());

        // Check pairing pass and token
        assert!(!secrets.pairing_pass().is_empty());
        assert_eq!(
            secrets.pairing_token().len(),
            std::mem::size_of::<PairingToken>()
        );

        // Check v3.1 specific fields
        assert_eq!(secrets.version(), AppletVersion::V3_1);
        assert_eq!(secrets.max_pin_attempts(), 4);
        assert_eq!(secrets.max_puk_attempts(), 6);
        assert!(secrets.duress_pin().is_some());
        assert_eq!(secrets.duress_pin().unwrap().len(), PIN_LENGTH);
    }

    #[test]
    fn test_to_bytes_legacy() {
        let secrets = Secrets::new("123456", "123456789012", "test-pairing-pass");
        let bytes = secrets.to_bytes();

        // Legacy format: PIN (6) + PUK (12) + pairing token (32)
        let expected_length = PIN_LENGTH + PUK_LENGTH + std::mem::size_of::<PairingToken>();
        assert_eq!(bytes.len(), expected_length);

        // Verify PIN bytes
        assert_eq!(&bytes[0..PIN_LENGTH], "123456".as_bytes());

        // Verify PUK bytes
        assert_eq!(
            &bytes[PIN_LENGTH..(PIN_LENGTH + PUK_LENGTH)],
            "123456789012".as_bytes()
        );

        // Verify pairing token
        let token_start = PIN_LENGTH + PUK_LENGTH;
        let token_end = token_start + std::mem::size_of::<PairingToken>();
        assert_eq!(
            &bytes[token_start..token_end],
            secrets.pairing_token().as_slice()
        );
    }

    #[test]
    fn test_to_bytes_v3_1() {
        let secrets = Secrets::new_v3_1(
            "123456",
            "123456789012",
            "test-pairing-pass",
            5,
            7,
            Some("654321".to_string()),
        );
        let bytes = secrets.to_bytes();

        // v3.1 format: PIN (6) + PUK (12) + pairing token (32) + max PIN attempts (1) + max PUK attempts (1) + duress PIN (6)
        let expected_length =
            PIN_LENGTH + PUK_LENGTH + std::mem::size_of::<PairingToken>() + 1 + 1 + PIN_LENGTH;
        assert_eq!(bytes.len(), expected_length);

        // Verify PIN bytes
        assert_eq!(&bytes[0..PIN_LENGTH], "123456".as_bytes());

        // Verify PUK bytes
        assert_eq!(
            &bytes[PIN_LENGTH..(PIN_LENGTH + PUK_LENGTH)],
            "123456789012".as_bytes()
        );

        // Verify pairing token
        let token_start = PIN_LENGTH + PUK_LENGTH;
        let token_end = token_start + std::mem::size_of::<PairingToken>();
        assert_eq!(
            &bytes[token_start..token_end],
            secrets.pairing_token().as_slice()
        );

        // Verify max PIN attempts
        assert_eq!(bytes[token_end], 5);

        // Verify max PUK attempts
        assert_eq!(bytes[token_end + 1], 7);

        // Verify duress PIN
        let duress_start = token_end + 2;
        let duress_end = duress_start + PIN_LENGTH;
        assert_eq!(&bytes[duress_start..duress_end], "654321".as_bytes());
    }

    #[test]
    fn test_to_bytes_v3_1_default_duress() {
        let secrets = Secrets::new_v3_1("123456", "123456789012", "test-pairing-pass", 5, 7, None);
        let bytes = secrets.to_bytes();

        // v3.1 format with default duress PIN (first half of PUK)
        let expected_length =
            PIN_LENGTH + PUK_LENGTH + std::mem::size_of::<PairingToken>() + 1 + 1 + PIN_LENGTH;
        assert_eq!(bytes.len(), expected_length);

        // Calculate offset for duress PIN
        let duress_start = PIN_LENGTH + PUK_LENGTH + std::mem::size_of::<PairingToken>() + 2;
        let duress_end = duress_start + PIN_LENGTH;

        // Verify duress PIN is first half of PUK
        assert_eq!(&bytes[duress_start..duress_end], "123456".as_bytes());
    }

    #[test]
    fn test_generate_pairing_pass() {
        let pass = generate_pairing_pass();
        assert!(!pass.is_empty());

        // Should be valid base64url
        assert!(BASE64_URL_SAFE_NO_PAD.decode(&pass).is_ok());
    }

    #[test]
    fn test_generate_pairing_token() {
        let token = generate_pairing_token("test-pass");
        assert_eq!(token.len(), std::mem::size_of::<PairingToken>());

        // Same input should generate same token
        let token2 = generate_pairing_token("test-pass");
        assert_eq!(token, token2);
    }
}
