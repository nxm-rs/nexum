//! Executor for APDU command execution
//!
//! This module provides executors that combine card transports with
//! command processors to handle APDU command execution.

pub mod ext;

use std::fmt;
use std::marker::PhantomData;

use crate::Response;
use crate::error::Error as DefaultError;
use crate::processor::{ProcessorError, SecureProtocolError};
use crate::response::error::ResponseError;
use bytes::Bytes;
use tracing::{debug, instrument, trace};

use crate::command::{ApduCommand, Command};
use crate::processor::{
    CommandProcessor,
    secure::{SecureChannelProvider, SecurityLevel},
};
use crate::transport::{CardTransport, TransportError};

/// Trait for APDU executor error types
pub trait ApduExecutorErrors: Send + Sync + fmt::Debug {
    /// Error type used by the executor
    type Error: From<ResponseError>
        + From<ProcessorError>
        + From<TransportError>
        + From<SecureProtocolError>
        + fmt::Debug;
}

// Implement for our default error type
impl ApduExecutorErrors for DefaultError {
    type Error = Self;
}

// Re-export extension traits
pub use ext::{ResponseAwareExecutor, SecureChannelExecutor};

/// Trait for APDU command execution
pub trait Executor: ApduExecutorErrors + Send + Sync + fmt::Debug {
    /// Transmit a raw APDU command
    ///
    /// This is the lowest level public transmission method.
    #[instrument(level = "trace", skip(self), fields(executor = std::any::type_name::<Self>()))]
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Self::Error> {
        trace!(command = ?hex::encode(command), "Transmitting raw command");
        let response = self.do_transmit_raw(command);
        match &response {
            Ok(bytes) => {
                trace!(response = ?hex::encode(bytes), "Received raw response");
            }
            Err(err) => {
                debug!(error = ?err, "Error during raw transmission");
            }
        }
        response
    }

    /// Internal implementation of transmit_raw
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Self::Error>;

    /// Transmit a generic Command and return a Response
    ///
    /// This is the mid-level transmission method that works with Command and Response objects.
    #[instrument(level = "trace", skip(self), fields(executor = std::any::type_name::<Self>()))]
    fn transmit(&mut self, command: &Command) -> Result<Response, Self::Error> {
        trace!(command = ?command, "Transmitting command");
        let command_bytes = command.to_bytes();
        let response_bytes = self.transmit_raw(&command_bytes)?;
        let response = Response::from_bytes(&response_bytes)?;
        trace!(response = ?response, "Received response");
        Ok(response)
    }

    /// Execute a typed APDU command and return the Result type (Success variant or Error)
    ///
    /// This method returns the command's Result type (not Response enum) for more
    /// idiomatic error handling with the ? operator.
    fn execute<C>(&mut self, command: &C) -> Result<C::Success, Self::Error>
    where
        C: ApduCommand,
    {
        // Check security level requirement
        let required_level = command.required_security_level();
        let current_level = self.security_level();

        // Verify security level is sufficient
        if !required_level.is_none() && !current_level.satisfies(&required_level) {
            return Err(SecureProtocolError::InsufficientSecurityLevel.into());
        }

        // Get command bytes and transmit
        let command_bytes = command.to_bytes();
        let response_bytes = self.transmit_raw(&command_bytes)?;

        // Parse the response bytes directly into the success type
        // or convert errors to the executor's error type
        C::parse_response(Response::from_bytes(&response_bytes)?)
            .map_err(|_| SecureProtocolError::Other("Command execution failed".to_string()).into())
    }

    /// Get current security level
    fn security_level(&self) -> SecurityLevel;

    /// Reset the executor, including the transport
    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// Card executor implementation that combines a transport with optional command processors
#[derive(Debug)]
pub struct CardExecutor<T: CardTransport, E: ApduExecutorErrors = DefaultError> {
    /// The transport used for communication
    transport: T,
    /// Command processors chain (last one processes first)
    processors: Vec<Box<dyn CommandProcessor>>,
    /// The last response received
    last_response: Option<Bytes>,
    /// Phantom data for error type
    _error: PhantomData<E>,
}

// Implement the trait for CardExecutor
impl<T: CardTransport, E: ApduExecutorErrors> ApduExecutorErrors for CardExecutor<T, E> {
    type Error = E::Error;
}

// Define all methods for any error type
impl<T: CardTransport, E: ApduExecutorErrors> CardExecutor<T, E> {
    /// Create a new card executor with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            processors: Vec::new(),
            last_response: None,
            _error: PhantomData,
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
        provider: &dyn SecureChannelProvider,
    ) -> Result<(), SecureProtocolError> {
        debug!("Opening secure channel");

        // Create the secure channel
        let secure_channel = provider.create_secure_channel(&mut self.transport)?;

        // Now secure_channel is Box<dyn SecureChannel>, which implements CommandProcessor
        self.processors.push(secure_channel);

        Ok(())
    }
}

impl<T: CardTransport, E: ApduExecutorErrors + fmt::Debug> Executor for CardExecutor<T, E> {
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Self::Error> {
        // Pass command to the command processor chain if any are active
        if !self.processors.is_empty() {
            // Try to parse the command bytes into a Command
            if let Ok(command_obj) = Command::from_bytes(command) {
                // Find the first active processor (process from end of chain)
                for i in (0..self.processors.len()).rev() {
                    if self.processors[i].is_active() {
                        // Process the command through this processor
                        let processor = &mut self.processors[i];
                        let response =
                            processor.process_command(&command_obj, &mut self.transport)?;
                        let response_bytes: Bytes = response.into();
                        self.last_response = Some(response_bytes.clone());
                        return Ok(response_bytes);
                    }
                }
            }
        }

        // If no processors or parsing failed, use transport directly
        let response = self.transport.transmit_raw(command)?;
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
        let mut executor: CardExecutor<MockTransport> = CardExecutor::new(transport);

        let response = executor.transmit_raw(&[0x00, 0xA4, 0x04, 0x00]).unwrap();
        assert_eq!(response.as_ref(), &[0x90, 0x00]);
    }

    #[test]
    fn test_executor_with_processor() {
        let transport = MockTransport::with_response(Bytes::from_static(&[0x90, 0x00]));
        let mut executor: CardExecutor<MockTransport> = CardExecutor::new(transport);

        // Add an identity processor
        executor.add_processor(Box::new(IdentityProcessor));

        let response = executor.transmit_raw(&[0x00, 0xA4, 0x04, 0x00]).unwrap();
        assert_eq!(response.as_ref(), &[0x90, 0x00]);
    }
}
