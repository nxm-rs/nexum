//! APDU response definitions and traits
//!
//! This module provides types and traits for working with APDU responses
//! according to ISO/IEC 7816-4.

pub mod error;
pub mod status;
pub mod utils;

use bytes::{BufMut, Bytes, BytesMut};
use tracing::trace;

use error::{ResponseError, StatusError};
use status::StatusWord;

/// Trait for APDU responses
pub trait ApduResponse: Sized {
    /// Get the response payload data
    fn payload(&self) -> &Option<Bytes>;

    /// Get the status word
    fn status(&self) -> StatusWord;

    /// Check if the response indicates success
    fn is_success(&self) -> bool {
        self.status().is_success()
    }

    /// Create from raw APDU response data
    fn from_bytes(data: &Bytes) -> Result<Self, ResponseError>;
}

/// Trait for types that can be created from APDU response data
pub trait FromApduResponse: Sized {
    /// Convert raw APDU response data to this type
    fn from_response(data: &Bytes) -> Result<Self, ResponseError>;
}

/// Basic APDU response structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Response payload data
    payload: Option<Bytes>,
    /// Status word
    status: StatusWord,
}

impl Response {
    /// Create a new response with payload and status
    pub fn new(payload: Option<Bytes>, status: impl Into<StatusWord>) -> Self {
        Self {
            payload,
            status: status.into(),
        }
    }

    /// Create a success response
    pub const fn success(payload: Option<Bytes>) -> Self {
        Self {
            payload,
            status: StatusWord::new(0x90, 0x00),
        }
    }

    /// Create an error response from a status word
    pub fn error(status: impl Into<StatusWord>) -> Self {
        Self {
            payload: None,
            status: status.into(),
        }
    }

    /// Parse response from raw bytes (including status word)
    pub fn from_bytes(data: &Bytes) -> Result<Self, ResponseError> {
        let (status, payload) = utils::extract_status_and_payload(data)?;

        trace!(
            sw1 = format_args!("{:#04x}", status.sw1),
            sw2 = format_args!("{:#04x}", status.sw2),
            payload_len = payload.as_ref().map_or(0, |p| p.len()),
            "Parsed APDU response"
        );

        Ok(Self { payload, status })
    }

    /// Get the status word as a tuple (SW1, SW2)
    pub const fn status_tuple(&self) -> (u8, u8) {
        (self.status.sw1, self.status.sw2)
    }

    /// Convert to a bytes result
    pub fn into_bytes_result(self) -> Result<Option<Bytes>, StatusError> {
        if self.is_success() {
            Ok(self.payload)
        } else {
            Err(StatusError::new(self.status.sw1, self.status.sw2))
        }
    }

    /// Convert to a bytes reference result
    pub fn as_bytes_result(&self) -> Result<&Option<Bytes>, ResponseError> {
        if self.is_success() {
            Ok(&self.payload)
        } else {
            Err(StatusError::new(self.status.sw1, self.status.sw2).into())
        }
    }
}

impl ApduResponse for Response {
    fn payload(&self) -> &Option<Bytes> {
        &self.payload
    }

    fn status(&self) -> StatusWord {
        self.status
    }

    fn from_bytes(data: &Bytes) -> Result<Self, ResponseError> {
        Response::from_bytes(data)
    }
}

impl TryFrom<&[u8]> for Response {
    type Error = ResponseError;

    fn try_from(data: &[u8]) -> Result<Self, ResponseError> {
        Self::from_bytes(&Bytes::copy_from_slice(data))
    }
}

// Allow creating Response from Bytes for compatibility with executor
impl TryFrom<Bytes> for Response {
    type Error = ResponseError;

    fn try_from(data: Bytes) -> Result<Self, ResponseError> {
        Self::from_bytes(&data)
    }
}

impl From<Response> for Bytes {
    fn from(response: Response) -> Self {
        let mut buf = BytesMut::with_capacity(response.payload.as_ref().map_or(0, |p| p.len()) + 2);
        if let Some(payload) = response.payload {
            buf.put_slice(&payload);
        }
        buf.put_u8(response.status.sw1);
        buf.put_u8(response.status.sw2);
        buf.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        let data = Some(Bytes::from_static(&[0x01, 0x02, 0x03][..]));
        let resp = Response::new(data, (0x90, 0x00));
        assert_eq!(
            resp.payload(),
            &Some(Bytes::from_static(&[0x01, 0x02, 0x03]))
        );
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
        assert!(resp.is_success());
    }

    #[test]
    fn test_response_from_bytes() {
        let data = Bytes::from_static(&[0x01, 0x02, 0x03, 0x90, 0x00]);
        let resp = Response::from_bytes(&data).unwrap();
        assert_eq!(
            resp.payload().as_ref().unwrap().as_ref(),
            &[0x01, 0x02, 0x03]
        );
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
        assert!(resp.is_success());

        let data = Bytes::from_static(&[0x90, 0x00]);
        let resp = Response::from_bytes(&data).unwrap();
        assert!(resp.payload().is_none());
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
        assert!(resp.is_success());

        let data = Bytes::from_static(&[0x01]);
        assert!(Response::from_bytes(&data).is_err());
    }

    #[test]
    fn test_response_into_result() {
        let data = Bytes::from_static(&[0x01, 0x02, 0x03][..]);
        let success = Response::success(Some(data));

        let result = success.into_bytes_result();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().as_ref(),
            Some(&Bytes::from_static(&[0x01, 0x02, 0x03]))
        );

        let error = Response::error((0x6A, 0x82));
        let result = error.into_bytes_result();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status.to_u16(), 0x6A82);
    }
}
