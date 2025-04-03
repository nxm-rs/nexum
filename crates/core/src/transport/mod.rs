//! Transport traits for APDU communication with cards
//!
//! This module provides abstractions for communicating with smart cards through
//! different transport mechanisms.

pub mod error;

use std::fmt;

use bytes::Bytes;
pub use error::TransportError;
use tracing::{debug, trace};

/// Trait for basic card transports
///
/// A transport is responsible for sending and receiving raw APDU bytes.
/// It has no knowledge of command structure, secure channels, or protocol details.
pub trait CardTransport: Send + Sync + fmt::Debug {
    /// Error type returned by the transport
    type Error: Into<crate::Error> + fmt::Debug;

    /// Send raw APDU bytes to card and return response bytes
    ///
    /// This method should handle the low-level communication with the card
    /// but should not interpret the contents or handle protocol-specific
    /// operations like GET RESPONSE.
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Self::Error> {
        trace!(command = ?hex::encode(command), "Transmitting raw command");
        let result = self.do_transmit_raw(command);
        match &result {
            Ok(response) => {
                trace!(response = ?hex::encode(response), "Received raw response");
            }
            Err(e) => {
                debug!(error = ?e, "Transport error during transmission");
            }
        }
        result
    }

    /// Internal implementation of transmit_raw
    /// This is the method that concrete implementations should override
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Self::Error>;

    /// Check if the transport is connected to a physical card
    fn is_connected(&self) -> bool;

    /// Reset the transport connection
    fn reset(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct MockTransport {
    /// Mock responses to return
    pub responses: Vec<Bytes>,
    /// Commands that were sent
    pub commands: Vec<Bytes>,
    /// Whether the transport is connected
    pub connected: bool,
}

#[cfg(test)]
impl MockTransport {
    /// Create a new mock transport with the given responses
    pub fn new(responses: Vec<Bytes>) -> Self {
        Self {
            responses,
            commands: Vec::new(),
            connected: true,
        }
    }

    /// Create a new mock transport that always returns the given response
    pub fn with_response(response: Bytes) -> Self {
        Self {
            responses: vec![response],
            commands: Vec::new(),
            connected: true,
        }
    }

    /// Create a new mock transport that always returns success (90 00)
    pub fn with_success() -> Self {
        Self::with_response(Bytes::from_static(&[0x90, 0x00]))
    }
}

#[cfg(test)]
impl CardTransport for MockTransport {
    type Error = TransportError;

    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Self::Error> {
        if !self.connected {
            return Err(TransportError::Connection);
        }

        self.commands.push(Bytes::copy_from_slice(command));

        if self.responses.is_empty() {
            return Err(TransportError::Transmission)?;
        }

        // Either clone the single response or take the next one
        if self.responses.len() == 1 {
            Ok(self.responses[0].clone())
        } else {
            Ok(self.responses.remove(0))
        }
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.connected = true;
        self.commands.clear();
        Ok(())
    }
}
