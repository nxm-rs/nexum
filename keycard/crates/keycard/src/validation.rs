//! Input validation utilities for Keycard application
//!
//! This module provides validation functions for user input related to
//! keycard operations, such as PIN validation and pairing information validation.

use alloy_primitives::hex;
use tracing::warn;

/// Error type for input validation failures
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "std", error("Invalid input: {0}"))]
pub enum ValidationError {
    /// The input was not the expected length
    #[error("Input has incorrect length: expected {expected}, got {actual}")]
    IncorrectLength {
        /// Expected length
        expected: usize,
        /// Actual length
        actual: usize,
    },

    /// The input contained invalid characters
    #[error("Input contains invalid characters")]
    InvalidCharacters,

    /// The input was out of the allowed range
    #[error("Input is out of allowed range: value {value}, min {min}, max {max}")]
    OutOfRange {
        /// The value that was out of range
        value: usize,
        /// Minimum allowed value
        min: usize,
        /// Maximum allowed value
        max: usize,
    },

    /// Generic validation error with a message
    #[error("{0}")]
    Message(String),
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validates a PIN input
///
/// A valid PIN must be exactly 6 digits long.
///
/// # Arguments
/// * `pin` - The PIN to validate
///
/// # Returns
/// * `Ok(pin)` - If the PIN is valid
/// * `Err` - If the PIN is invalid
pub fn validate_pin(pin: &str) -> ValidationResult<String> {
    // Check length
    if pin.len() != 6 {
        return Err(ValidationError::IncorrectLength {
            expected: 6,
            actual: pin.len(),
        });
    }

    // Check that all characters are digits
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(ValidationError::InvalidCharacters);
    }

    Ok(pin.to_string())
}

/// Validates a pairing index
///
/// A valid pairing index must be between 0 and 99 (inclusive).
///
/// # Arguments
/// * `index` - The pairing index to validate
///
/// # Returns
/// * `Ok(index)` - If the index is valid
/// * `Err` - If the index is invalid
pub fn validate_pairing_index(index: u8) -> ValidationResult<u8> {
    // Check range (0-99)
    if index > 99 {
        return Err(ValidationError::OutOfRange {
            value: index as usize,
            min: 0,
            max: 99,
        });
    }

    Ok(index)
}

/// Validates and decodes a hex string to a 32-byte array
///
/// # Arguments
/// * `hex_str` - The hex string to validate and decode
///
/// # Returns
/// * `Ok([u8; 32])` - If the hex string is valid and decodes to 32 bytes
/// * `Err` - If the hex string is invalid or doesn't decode to 32 bytes
pub fn validate_and_decode_hex(hex_str: &str) -> ValidationResult<[u8; 32]> {
    // Trim and remove any whitespace
    let hex_str = hex_str.trim().replace(" ", "");

    // Check if it's valid hex
    if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValidationError::InvalidCharacters);
    }

    // Check length is appropriate for 32 bytes (64 hex chars)
    if hex_str.len() != 64 {
        return Err(ValidationError::IncorrectLength {
            expected: 64,
            actual: hex_str.len(),
        });
    }

    // Try to decode the hex string
    match hex::decode(&hex_str) {
        Ok(bytes) => {
            // Convert to fixed size array
            if bytes.len() != 32 {
                return Err(ValidationError::IncorrectLength {
                    expected: 32,
                    actual: bytes.len(),
                });
            }

            let mut array = [0u8; 32];
            array.copy_from_slice(&bytes);
            Ok(array)
        }
        Err(_) => Err(ValidationError::Message(
            "Failed to decode hex string".to_string(),
        )),
    }
}

/// Attempts to get a valid PIN from the user using the provided input function
///
/// This function will keep prompting the user until a valid PIN is entered or
/// a maximum number of attempts is reached.
///
/// # Arguments
/// * `input_fn` - Function to request input from the user
/// * `attempts` - Maximum number of attempts (defaults to 3)
///
/// # Returns
/// * `String` - The validated PIN
pub fn get_valid_pin<F>(input_fn: &F, attempts: usize) -> String
where
    F: Fn(&str) -> String,
{
    let max_attempts = if attempts > 0 { attempts } else { 3 };

    for attempt in 0..max_attempts {
        let prompt = if attempt == 0 {
            "Enter PIN (6 digits)".to_string()
        } else {
            format!(
                "Invalid PIN. Enter PIN (6 digits, attempt {}/{})",
                attempt + 1,
                max_attempts
            )
        };

        let pin = input_fn(&prompt);

        match validate_pin(&pin) {
            Ok(valid_pin) => return valid_pin,
            Err(e) => warn!("PIN validation failed: {}", e),
        }
    }

    // If we get here, the user failed all attempts, return a default value
    warn!("Maximum PIN entry attempts reached, using default value");
    panic!("Maximum attempts reached for requesting valid input");
}

/// Attempts to get a valid pairing key from the user using the provided input function
///
/// This function will keep prompting the user until a valid key is entered or
/// a maximum number of attempts is reached.
///
/// # Arguments
/// * `input_fn` - Function to request input from the user
/// * `attempts` - Maximum number of attempts (defaults to 3)
///
/// # Returns
/// * `[u8; 32]` - The validated and decoded key
pub fn get_valid_pairing_key<F>(input_fn: &F, attempts: usize) -> [u8; 32]
where
    F: Fn(&str) -> String,
{
    let max_attempts = if attempts > 0 { attempts } else { 3 };

    for attempt in 0..max_attempts {
        let prompt = if attempt == 0 {
            "Enter pairing key (64 hex characters)".to_string()
        } else {
            format!(
                "Invalid key. Enter pairing key (64 hex characters, attempt {}/{})",
                attempt + 1,
                max_attempts
            )
        };

        let key_hex = input_fn(&prompt);

        match validate_and_decode_hex(&key_hex) {
            Ok(key) => return key,
            Err(e) => warn!("Pairing key validation failed: {}", e),
        }
    }

    // If we get here, the user failed all attempts, return a default value
    warn!("Maximum pairing key entry attempts reached, using default value");
    panic!("Maximum attempts reached for requesting valid input")
}

/// Attempts to get a valid pairing index from the user using the provided input function
///
/// This function will keep prompting the user until a valid index is entered or
/// a maximum number of attempts is reached.
///
/// # Arguments
/// * `input_fn` - Function to request input from the user
/// * `attempts` - Maximum number of attempts (defaults to 3)
///
/// # Returns
/// * `u8` - The validated pairing index
pub fn get_valid_pairing_index<F>(input_fn: &F, attempts: usize) -> u8
where
    F: Fn(&str) -> String,
{
    let max_attempts = if attempts > 0 { attempts } else { 3 };

    for attempt in 0..max_attempts {
        let prompt = if attempt == 0 {
            "Enter pairing index (0-99)".to_string()
        } else {
            format!(
                "Invalid index. Enter pairing index (0-99, attempt {}/{})",
                attempt + 1,
                max_attempts
            )
        };

        let index_str = input_fn(&prompt);

        // Try to parse as a number
        match index_str.trim().parse::<u8>() {
            Ok(index) => match validate_pairing_index(index) {
                Ok(valid_index) => return valid_index,
                Err(e) => warn!("Pairing index validation failed: {}", e),
            },
            Err(_) => warn!("Failed to parse pairing index as a number"),
        }
    }

    // If we get here, the user failed all attempts, return a default value
    warn!("Maximum pairing index entry attempts reached, using default value");
    panic!("Maximum attempts reached for requesting valid input");
}
