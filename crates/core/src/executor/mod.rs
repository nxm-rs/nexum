//! Executor for APDU command execution
//!
//! This module provides executors that combine card transports with
//! command processors to handle APDU command execution.

pub mod ext;

use std::fmt;

use bytes::Bytes;
use tracing::{debug, instrument, trace};

use crate::command::{ApduCommand, Command};
use crate::processor::{
    CommandProcessor, ProcessorError,
    secure::{SecureChannelProvider, SecurityLevel},
};
use crate::transport::{CardTransport, TransportError};

// Re-export extension traits
pub use ext::{ResponseAwareExecutor, SecureChannelExecutor};

/// Trait for APDU command execution
pub trait Executor: Send + Sync + fmt::Debug {
    /// Error type returned by the executor
    type Error: Into<crate::Error> + fmt::Debug;

    /// Transmit an APDU command
    ///
    /// This method handles protocol details including routing through
    /// command processors and secure channels if established.
    #[instrument(level = "trace", skip(self), fields(executor = std::any::type_name::<Self>()))]
    fn transmit(&mut self, command: &[u8]) -> Result<Bytes, Self::Error> {
        trace!(command = ?hex::encode(command), "Transmitting command");
        let response = self.do_transmit(command);
        match &response {
            Ok(bytes) => {
                trace!(response = ?hex::encode(bytes), "Received response");
            }
            Err(err) => {
                debug!(error = ?err, "Error during transmission");
            }
        }
        response
    }

    /// Internal implementation of transmit
    fn do_transmit(&mut self, command: &[u8]) -> Result<Bytes, Self::Error>;

    /// Execute a typed APDU command
    fn execute<C>(&mut self, command: &C) -> core::result::Result<C::Response, Self::Error>
    where
        C: ApduCommand,
        C::Response: TryFrom<Bytes>,
        <C::Response as TryFrom<Bytes>>::Error: Into<Self::Error>,
        Self::Error: Into<Self::Error>,
    {
        let command_bytes = command.to_bytes();
        let response_bytes = self.transmit(&command_bytes)?;
        C::Response::try_from(response_bytes).map_err(Into::into)
    }

    /// Get current security level
    fn security_level(&self) -> SecurityLevel;

    /// Reset the executor, including the transport
    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// Card executor implementation that combines a transport with optional command processors
#[derive(Debug)]
pub struct CardExecutor<T: CardTransport> {
    /// The transport used for communication
    transport: T,
    /// Command processors chain (last one processes first)
    processors: Vec<Box<dyn CommandProcessor<Error = ProcessorError>>>,
    /// The last response received
    last_response: Option<Bytes>,
}

impl<T: CardTransport<Error = TransportError>> CardExecutor<T> {
    /// Create a new card executor with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            processors: Vec::new(),
            last_response: None,
        }
    }

    /// Create a new card executor with transport and default command processors
    pub fn new_with_defaults(transport: T) -> Self {
        let mut executor = Self::new(transport);
        // Add standard GET RESPONSE handler
        executor.add_processor(Box::new(crate::processor::GetResponseProcessor::default()));
        executor
    }

    /// Get a reference to the underlying transport
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    /// Get a mutable reference to the underlying transport
    pub const fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Take ownership of the transport and return it
    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Add a command processor to the chain
    pub fn add_processor(&mut self, processor: Box<dyn CommandProcessor<Error = ProcessorError>>) {
        self.processors.push(processor);
    }

    /// Get the active command processors
    pub fn processors(&self) -> &[Box<dyn CommandProcessor<Error = ProcessorError>>] {
        &self.processors
    }

    /// Get mutable access to the command processors
    pub fn processors_mut(
        &mut self,
    ) -> &mut Vec<Box<dyn CommandProcessor<Error = ProcessorError>>> {
        &mut self.processors
    }

    /// Remove all command processors
    pub fn clear_processors(&mut self) {
        self.processors.clear();
    }

    /// Get the last response received
    pub const fn last_response(&self) -> Option<&Bytes> {
        self.last_response.as_ref()
    }

    /// Open a secure channel using the provided secure channel provider
    pub fn open_secure_channel(
        &mut self,
        provider: &dyn SecureChannelProvider<Error = ProcessorError>,
    ) -> Result<(), ProcessorError> {
        debug!("Opening secure channel");

        // Create the secure channel
        let secure_channel = provider.create_secure_channel(&mut self.transport)?;

        // Now secure_channel is Box<dyn SecureChannel>, which implements CommandProcessor
        self.processors.push(secure_channel);

        Ok(())
    }
}

impl<T: CardTransport<Error = TransportError>> Executor for CardExecutor<T> {
    type Error = crate::Error;

    fn do_transmit(&mut self, command: &[u8]) -> Result<Bytes, Self::Error> {
        // Parse the command bytes into a Command
        let command = Command::from_bytes(command)?;

        // If we have processors, use them
        if !self.processors.is_empty() {
            // Find the first active processor (process from end of chain)
            for i in (0..self.processors.len()).rev() {
                if self.processors[i].is_active() {
                    // Get mutable reference to the processor
                    let processor = &mut self.processors[i];

                    // Process the command through this processor
                    let response = processor.process_command(&command, &mut self.transport)?;

                    // Convert Response to Bytes for compatibility with existing API
                    let response_bytes: Bytes = response.into();

                    // Save response and return
                    self.last_response = Some(response_bytes.clone());
                    return Ok(response_bytes);
                }
            }
        }

        // If no processors or none active, use transport directly
        let command_bytes = command.to_bytes();
        let response = self.transport.transmit_raw(&command_bytes)?;
        self.last_response = Some(response.clone());
        Ok(response)
    }

    fn security_level(&self) -> SecurityLevel {
        // Return the highest security level from all active processors
        self.processors
            .iter()
            .filter(|p| p.is_active())
            .map(|p| p.security_level())
            .max()
            .unwrap_or(SecurityLevel::none())
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        // Reset the transport
        self.transport.reset()?;

        // Clear processors that depend on session state
        self.processors.clear();

        // Clear last response
        self.last_response = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::IdentityProcessor;
    use crate::transport::MockTransport;

    #[test]
    fn test_executor_basic_transmit() {
        let transport = MockTransport::with_response(Bytes::from_static(&[0x90, 0x00]));
        let mut executor = CardExecutor::new(transport);

        let response = executor.transmit(&[0x00, 0xA4, 0x04, 0x00]).unwrap();
        assert_eq!(response.as_ref(), &[0x90, 0x00]);
    }

    #[test]
    fn test_executor_with_processor() {
        let transport = MockTransport::with_response(Bytes::from_static(&[0x90, 0x00]));
        let mut executor = CardExecutor::new(transport);

        // Add an identity processor
        executor.add_processor(Box::new(IdentityProcessor));

        let response = executor.transmit(&[0x00, 0xA4, 0x04, 0x00]).unwrap();
        assert_eq!(response.as_ref(), &[0x90, 0x00]);
    }
}
