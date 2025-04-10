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
    /// Send raw APDU bytes to card and return response bytes
    ///
    /// This is the lowest level transmission method that should only deal with raw bytes.
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, TransportError> {
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
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, TransportError>;

    /// Check if the transport is connected to a physical card
    fn is_connected(&self) -> bool;

    /// Reset the transport connection
    fn reset(&mut self) -> Result<(), TransportError>;
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
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, TransportError> {
        if !self.connected {
            return Err(TransportError::Connection);
        }

        self.commands.push(Bytes::copy_from_slice(command));

        if self.responses.is_empty() {
            return Err(TransportError::Transmission);
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

    fn reset(&mut self) -> Result<(), TransportError> {
        self.connected = true;
        self.commands.clear();
        Ok(())
    }
}
