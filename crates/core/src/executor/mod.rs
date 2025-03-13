//! Executor for APDU command execution
//!
//! This module provides executors that combine card transports with
//! command processors to handle APDU command execution.

pub mod error;
pub mod ext;

use bytes::Bytes;
use core::fmt;
use tracing::{debug, instrument, trace};

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::command::{ApduCommand, Command};
use crate::processor::CommandProcessor;
use crate::processor::secure::SecurityLevel;
use crate::transport::CardTransport;
use crate::{Error, Result};

// Re-export extension traits
pub use ext::{ResponseAwareExecutor, SecureChannelExecutor};

/// Trait for APDU command execution
pub trait Executor: Send + Sync + fmt::Debug {
    /// Transmit an APDU command
    ///
    /// This method handles protocol details including routing through
    /// command processors and secure channels if established.
    #[instrument(level = "trace", skip(self), fields(executor = std::any::type_name::<Self>()))]
    fn transmit(&mut self, command: &[u8]) -> Result<Bytes> {
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
    fn do_transmit(&mut self, command: &[u8]) -> Result<Bytes>;

    /// Execute a typed APDU command
    fn execute<C: ApduCommand>(
        &mut self,
        command: &C,
    ) -> core::result::Result<C::Response, C::Error>
    where
        Error: Into<C::Error>,
        C::Response: TryFrom<Bytes, Error = C::Error>,
    {
        let command_bytes = command.to_bytes();
        let response_bytes = self.transmit(&command_bytes).map_err(Into::into)?;
        C::Response::try_from(response_bytes)
    }

    /// Get current security level
    fn security_level(&self) -> SecurityLevel;

    /// Reset the executor, including the transport
    fn reset(&mut self) -> Result<()>;
}

/// Card executor implementation that combines a transport with optional command processors
#[derive(Debug)]
pub struct CardExecutor<T: CardTransport> {
    /// The transport used for communication
    transport: T,
    /// Command processors chain (last one processes first)
    processors: Vec<Box<dyn CommandProcessor>>,
    /// The last response received
    last_response: Option<Bytes>,
}

impl<T: CardTransport> CardExecutor<T> {
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
    pub fn add_processor(&mut self, processor: Box<dyn CommandProcessor>) {
        self.processors.push(processor);
    }

    /// Get the active command processors
    pub fn processors(&self) -> &[Box<dyn CommandProcessor>] {
        &self.processors
    }

    /// Get mutable access to the command processors
    pub fn processors_mut(&mut self) -> &mut Vec<Box<dyn CommandProcessor>> {
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
        provider: &dyn crate::processor::secure::SecureChannelProvider,
    ) -> Result<()> {
        debug!("Opening secure channel");

        // Create the secure channel
        let secure_channel = provider
            .create_secure_channel(&mut self.transport)
            .map_err(Error::Processor)?;

        // Add it to our processors
        self.add_processor(secure_channel);

        Ok(())
    }
}

impl<T: CardTransport> Executor for CardExecutor<T> {
    fn do_transmit(&mut self, command: &[u8]) -> Result<Bytes> {
        // Parse the command bytes into a Command
        let command = match Command::from_bytes(command) {
            Ok(cmd) => cmd,
            Err(e) => return Err(Error::Command(e)),
        };

        // If we have processors, use them
        if !self.processors.is_empty() {
            // Find the first active processor (process from end of chain)
            for i in (0..self.processors.len()).rev() {
                if self.processors[i].is_active() {
                    // Get mutable reference to the processor
                    let processor = &mut self.processors[i];

                    // Process the command through this processor
                    let response = processor
                        .process_command(&command, &mut self.transport)
                        .map_err(Error::Processor)?;

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
        let response = self
            .transport
            .transmit_raw(&command_bytes)
            .map_err(Error::Transport)?;
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
            .unwrap_or(SecurityLevel::NoSecurity)
    }

    fn reset(&mut self) -> Result<()> {
        // Reset the transport
        self.transport.reset().map_err(Error::Transport)?;

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
