//! APDU response definitions and traits
//!
//! This module provides types and traits for working with APDU responses
//! according to ISO/IEC 7816-4.

pub mod error;
pub mod status;
pub mod utils;

use std::fmt;

use bytes::{BufMut, Bytes, BytesMut};
use tracing::trace;

use error::{ResponseError, StatusError};
use status::StatusWord;

/// Trait for APDU responses
pub trait ApduResponse: Sized {
    /// Error type returned by the response
    type Error: Into<crate::Error> + fmt::Debug;

    /// Get the response payload data
    fn payload(&self) -> &[u8];

    /// Get the status word
    fn status(&self) -> StatusWord;

    /// Check if the response indicates success
    fn is_success(&self) -> bool {
        self.status().is_success()
    }

    /// Create from raw APDU response data
    fn from_bytes(data: &[u8]) -> Result<Self, Self::Error>;
}

/// Trait for types that can be created from APDU response data
pub trait FromApduResponse: Sized {
    /// Error that can occur during conversion
    type Error: Into<crate::Error> + fmt::Debug;

    /// Convert raw APDU response data to this type
    fn from_response(data: &[u8]) -> core::result::Result<Self, Self::Error>;
}

/// Basic APDU response structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Response payload data
    payload: Bytes,
    /// Status word
    status: StatusWord,
}

impl Response {
    /// Create a new response with payload and status
    pub fn new(payload: impl Into<Bytes>, status: impl Into<StatusWord>) -> Self {
        Self {
            payload: payload.into(),
            status: status.into(),
        }
    }

    /// Create a success response (SW=9000)
    pub fn success(payload: impl Into<Bytes>) -> Self {
        Self {
            payload: payload.into(),
            status: StatusWord::new(0x90, 0x00),
        }
    }

    /// Create an error response from a status word
    pub fn error(status: impl Into<StatusWord>) -> Self {
        Self {
            payload: Bytes::new(),
            status: status.into(),
        }
    }

    /// Parse response from raw bytes (including status word)
    pub fn from_bytes(data: &[u8]) -> Result<Self, ResponseError> {
        let (status, payload) = utils::extract_status_and_payload(data)?;

        trace!(
            sw1 = format_args!("{:#04x}", status.sw1),
            sw2 = format_args!("{:#04x}", status.sw2),
            payload_len = payload.len(),
            "Parsed APDU response"
        );

        Ok(Self {
            payload: Bytes::copy_from_slice(payload),
            status,
        })
    }

    /// Get the status word as a tuple (SW1, SW2)
    pub const fn status_tuple(&self) -> (u8, u8) {
        (self.status.sw1, self.status.sw2)
    }

    /// Convert to a bytes result
    pub fn into_bytes_result(self) -> core::result::Result<Bytes, StatusError> {
        if self.is_success() {
            Ok(self.payload)
        } else {
            Err(StatusError::new(self.status.sw1, self.status.sw2))
        }
    }

    /// Convert to a bytes reference result
    pub fn as_bytes_result(&self) -> core::result::Result<&[u8], StatusError> {
        if self.is_success() {
            Ok(&self.payload)
        } else {
            Err(StatusError::new(self.status.sw1, self.status.sw2))
        }
    }
}

impl ApduResponse for Response {
    type Error = ResponseError;

    fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn status(&self) -> StatusWord {
        self.status
    }

    fn from_bytes(data: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(data)
    }
}

impl TryFrom<&[u8]> for Response {
    type Error = ResponseError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(data)
    }
}

// Allow creating Response from Bytes for compatibility with executor
impl TryFrom<Bytes> for Response {
    type Error = ResponseError;

    fn try_from(data: Bytes) -> Result<Self, Self::Error> {
        Self::from_bytes(&data)
    }
}

impl From<Response> for Bytes {
    fn from(response: Response) -> Self {
        let mut buf = BytesMut::with_capacity(response.payload.len() + 2);
        buf.put_slice(&response.payload);
        buf.put_u8(response.status.sw1);
        buf.put_u8(response.status.sw2);
        buf.freeze()
    }
}

// Support converting to Vec for compatibility
impl From<Response> for Vec<u8> {
    fn from(response: Response) -> Self {
        let bytes: Bytes = response.into();
        bytes.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        let data = &[0x01, 0x02, 0x03][..];
        let resp = Response::new(Bytes::copy_from_slice(data), (0x90, 0x00));
        assert_eq!(resp.payload(), &[0x01, 0x02, 0x03]);
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
        assert!(resp.is_success());
    }

    #[test]
    fn test_response_from_bytes() {
        let data = [0x01, 0x02, 0x03, 0x90, 0x00];
        let resp = Response::from_bytes(&data).unwrap();
        assert_eq!(resp.payload(), &[0x01, 0x02, 0x03]);
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
        assert!(resp.is_success());

        let data = [0x90, 0x00];
        let resp = Response::from_bytes(&data).unwrap();
        assert_eq!(resp.payload(), &[]);
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
        assert!(resp.is_success());

        let data = [0x01];
        assert!(Response::from_bytes(&data).is_err());
    }

    #[test]
    fn test_response_into_result() {
        let data = &[0x01, 0x02, 0x03][..];
        let success = Response::success(Bytes::copy_from_slice(data));

        let result = success.into_bytes_result();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_ref(), &[0x01, 0x02, 0x03]);

        let error = Response::error((0x6A, 0x82));
        let result = error.into_bytes_result();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_word().to_u16(), 0x6A82);
    }
}
