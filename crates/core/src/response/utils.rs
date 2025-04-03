//! Utility functions for APDU response handling

use crate::response::error::ResponseError;
use crate::response::status::StatusWord;
use tracing::debug;

/// Extract status word (SW1, SW2) and payload from raw APDU response data
///
/// Returns a tuple containing:
/// - The status word as a tuple (SW1, SW2)
/// - The payload data (without the status word)
///
/// # Errors
/// Returns an error if the data is too short to contain a valid status word.
pub fn extract_response_parts(data: &[u8]) -> Result<((u8, u8), &[u8]), ResponseError> {
    if data.len() < 2 {
        debug!("Response too short: {} bytes", data.len());
        return Err(ResponseError::Incomplete);
    }

    let len = data.len();
    let sw1 = data[len - 2];
    let sw2 = data[len - 1];
    let payload = if len > 2 { &data[..len - 2] } else { &[] };

    Ok(((sw1, sw2), payload))
}

/// Extract status word as a StatusWord object and payload from raw APDU response data
///
/// Returns a tuple containing:
/// - The StatusWord object
/// - The payload data (without the status word)
///
/// # Errors
/// Returns an error if the data is too short to contain a valid status word.
pub fn extract_status_and_payload(data: &[u8]) -> Result<(StatusWord, &[u8]), ResponseError> {
    let ((sw1, sw2), payload) = extract_response_parts(data)?;
    Ok((StatusWord::new(sw1, sw2), payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_response_parts() {
        // Test with payload and status
        let data = [0x01, 0x02, 0x03, 0x90, 0x00];
        let result = extract_response_parts(&data).unwrap();
        assert_eq!(result.0, (0x90, 0x00));
        assert_eq!(result.1, &[0x01, 0x02, 0x03]);

        // Test with only status
        let data = [0x90, 0x00];
        let result = extract_response_parts(&data).unwrap();
        assert_eq!(result.0, (0x90, 0x00));
        assert_eq!(result.1, &[]);

        // Test with insufficient data
        let data = [0x90];
        assert!(extract_response_parts(&data).is_err());
    }

    #[test]
    fn test_extract_status_and_payload() {
        // Test with payload and status
        let data = [0x01, 0x02, 0x03, 0x90, 0x00];
        let result = extract_status_and_payload(&data).unwrap();
        assert_eq!(result.0, StatusWord::new(0x90, 0x00));
        assert_eq!(result.1, &[0x01, 0x02, 0x03]);

        // Test with only status
        let data = [0x90, 0x00];
        let result = extract_status_and_payload(&data).unwrap();
        assert_eq!(result.0, StatusWord::new(0x90, 0x00));
        assert_eq!(result.1, &[]);
    }
}
