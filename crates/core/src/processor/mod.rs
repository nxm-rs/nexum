//! Command processor for APDU commands
//!
//! This module provides traits and implementations for processing APDU commands
//! before they are sent to the card. This allows adding functionality like
//! secure channel encryption, extended APDUs, and other protocol features.

pub mod pipeline;
pub mod processors;

use std::fmt;

use crate::{transport::CardTransport, Command, Error, Response};

/// Trait for command processors
pub trait CommandProcessor: Send + Sync + fmt::Debug {
    /// Process a command and return a response
    ///
    /// The processor can modify the command before sending it to the card,
    /// or it can handle the command itself without sending anything.
    fn process_command_with_adapter(
        &self,
        command: &Command,
        adapter: &mut dyn TransportAdapterTrait,
    ) -> Result<Response, Error>;
}

/// Trait for transport adapters, making it possible to use dynamic dispatch
pub trait TransportAdapterTrait {
    /// Transmit a raw command over the transport
    fn transmit_raw(&mut self, command: &[u8]) -> Result<crate::Bytes, Error>;
    
    /// Reset the transport
    fn reset(&mut self) -> Result<(), Error>;
}

/// Transport adapter for processors to use
///
/// This adapter allows processors to use a transport without owning it.
#[derive(Debug)]
pub struct TransportAdapter<'a, T: CardTransport> {
    inner: &'a mut T,
}

impl<'a, T: CardTransport> TransportAdapter<'a, T> {
    /// Create a new transport adapter
    pub const fn new(transport: &'a mut T) -> Self {
        Self { inner: transport }
    }
}

impl<'a, T: CardTransport> TransportAdapterTrait for TransportAdapter<'a, T> {
    fn transmit_raw(&mut self, command: &[u8]) -> Result<crate::Bytes, Error> {
        self.inner.transmit_raw(command)
    }

    fn reset(&mut self) -> Result<(), Error> {
        self.inner.reset()
    }
}

impl<'a, T: CardTransport> CardTransport for TransportAdapter<'a, T> {
    fn transmit_raw(&mut self, command: &[u8]) -> Result<crate::Bytes, Error> {
        self.inner.transmit_raw(command)
    }

    fn reset(&mut self) -> Result<(), Error> {
        self.inner.reset()
    }
}