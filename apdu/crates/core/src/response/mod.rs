//! APDU response types and status codes
//!
//! This module provides types and enums for handling APDU responses
//! according to ISO/IEC 7816-4.

pub mod status;
pub mod utils;

use bytes::Bytes;

use crate::Error;
use status::StatusWord;

/// Core trait for parsing APDU responses
pub trait ApduResponse<T>: Sized {
    /// Parse the response from bytes
    fn from_response(response: Response) -> Result<T, Error>;
}

/// APDU response structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Response status word
    pub status: StatusWord,
    /// Response data (if any)
    pub data: Option<Bytes>,
}

impl Response {
    /// Create a new response
    pub const fn new(status: StatusWord, data: Option<Bytes>) -> Self {
        Self { status, data }
    }

    /// Create a success response (status 9000)
    pub const fn success(data: Option<Bytes>) -> Self {
        Self {
            status: StatusWord::new(0x90, 0x00),
            data,
        }
    }

    /// Create a response with just a status word
    pub const fn status_only(status: StatusWord) -> Self {
        Self { status, data: None }
    }

    /// Parse a response from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 2 {
            return Err(Error::parse("Response too short: must be at least 2 bytes"));
        }

        let data_len = bytes.len() - 2;
        let status = StatusWord::new(bytes[data_len], bytes[data_len + 1]);

        let data = if data_len > 0 {
            Some(Bytes::copy_from_slice(&bytes[0..data_len]))
        } else {
            None
        };

        Ok(Self { status, data })
    }

    /// Get the response payload
    pub const fn payload(&self) -> &Option<Bytes> {
        &self.data
    }

    /// Get the response payload as a slice
    pub fn payload_bytes(&self) -> Option<&[u8]> {
        self.data.as_ref().map(|b| b.as_ref())
    }

    /// Get the response status
    pub const fn status(&self) -> StatusWord {
        self.status
    }

    /// Check if the response is successful (9000)
    pub const fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Check if more data is available
    pub const fn more_data_available(&self) -> bool {
        self.status.is_more_data_available()
    }

    /// Get the number of additional bytes available
    pub const fn bytes_available(&self) -> Option<u8> {
        if self.status.sw1 == 0x61 {
            Some(self.status.sw2)
        } else {
            None
        }
    }

    /// Check if the response indicates wrong length
    pub const fn indicates_wrong_length(&self) -> bool {
        self.status.sw1 == 0x6C
    }

    /// Get the correct length if wrong length was indicated
    pub const fn correct_length(&self) -> Option<u8> {
        if self.indicates_wrong_length() {
            Some(self.status.sw2)
        } else {
            None
        }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Bytes {
        let result = self
            .data
            .as_ref()
            .map_or_else(Bytes::new, |data| data.clone());

        let status_bytes = [self.status.sw1, self.status.sw2];
        let mut combined = bytes::BytesMut::with_capacity(result.len() + 2);
        combined.extend_from_slice(&result);
        combined.extend_from_slice(&status_bytes);

        combined.freeze()
    }
}

/// Enable direct conversion from Response to Bytes
impl From<Response> for Bytes {
    fn from(response: Response) -> Self {
        response.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_success() {
        let resp = Response::success(Some(Bytes::from_static(&[0x01, 0x02, 0x03])));
        assert!(resp.is_success());
        assert_eq!(resp.status.sw1, 0x90);
        assert_eq!(resp.status.sw2, 0x00);
        assert_eq!(resp.data.as_ref().unwrap().as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_response_from_bytes() {
        // Response with data
        let bytes = &[0x01, 0x02, 0x03, 0x90, 0x00];
        let resp = Response::from_bytes(bytes).unwrap();
        assert!(resp.is_success());
        assert_eq!(resp.data.as_ref().unwrap().as_ref(), &[0x01, 0x02, 0x03]);

        // Response with just status
        let bytes = &[0x90, 0x00];
        let resp = Response::from_bytes(bytes).unwrap();
        assert!(resp.is_success());
        assert!(resp.data.is_none());

        // Error status
        let bytes = &[0x69, 0x85];
        let resp = Response::from_bytes(bytes).unwrap();
        assert!(!resp.is_success());
        assert_eq!(resp.status.sw1, 0x69);
        assert_eq!(resp.status.sw2, 0x85);
    }

    #[test]
    fn test_response_more_data() {
        let resp = Response::status_only(StatusWord::new(0x61, 0x23));
        assert!(resp.more_data_available());
        assert_eq!(resp.bytes_available(), Some(0x23));
    }

    #[test]
    fn test_response_wrong_length() {
        let resp = Response::status_only(StatusWord::new(0x6C, 0x10));
        assert!(resp.indicates_wrong_length());
        assert_eq!(resp.correct_length(), Some(0x10));
    }
}
