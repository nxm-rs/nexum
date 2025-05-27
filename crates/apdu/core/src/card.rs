//! Card executor implementation
//!
//! This module provides the CardExecutor implementation, which combines
//! card transports with command processors to handle APDU command execution.

use std::fmt;

use crate::Response;
use crate::command::{ApduCommand, Command};
use crate::error::{Error, ResultExt};
use crate::executor::{Executor, ResponseAwareExecutor, SecureChannelExecutor};
use crate::processor::CommandProcessor;
use crate::processor::pipeline::ProcessorPipeline;
use crate::secure_channel::{SecureChannel, SecurityLevel};
use bytes::Bytes;

/// Card executor implementation with a transport and processor pipeline
pub struct CardExecutor<T>
where
    T: crate::transport::CardTransport,
{
    /// The transport used for communication (could be a SecureChannel or raw transport)
    transport: T,
    /// Command processor pipeline
    pipeline: ProcessorPipeline,
    /// The last response received
    last_response: Option<Bytes>,
}

impl<T> fmt::Debug for CardExecutor<T>
where
    T: crate::transport::CardTransport,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CardExecutor")
            .field("transport", &self.transport)
            .field("pipeline", &self.pipeline)
            .field("last_response", &self.last_response)
            .finish()
    }
}

// Define all methods for CardExecutor
impl<T> CardExecutor<T>
where
    T: crate::transport::CardTransport,
{
    /// Create a new card executor with the given transport
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            pipeline: ProcessorPipeline::new(),
            last_response: None,
        }
    }

    /// Create a new card executor with transport and default command processors
    pub fn new_with_defaults(transport: T) -> Self {
        use crate::processor::processors::GetResponseProcessor;

        // Create executor
        let mut executor = Self::new(transport);

        // Add standard GET RESPONSE handler
        executor.add_processor(Box::new(GetResponseProcessor::default()));

        executor
    }

    /// Get a reference to the processor pipeline
    pub const fn pipeline(&self) -> &ProcessorPipeline {
        &self.pipeline
    }

    /// Get a mutable reference to the processor pipeline
    pub const fn pipeline_mut(&mut self) -> &mut ProcessorPipeline {
        &mut self.pipeline
    }

    /// Get the last response received
    pub const fn last_response_bytes(&self) -> Option<&Bytes> {
        self.last_response.as_ref()
    }

    /// Add a command processor to the pipeline
    pub fn add_processor(&mut self, processor: Box<dyn CommandProcessor>) -> &mut Self {
        self.pipeline.add_processor(processor);
        self
    }
}

// Implement the Executor trait for CardExecutor
impl<T> Executor for CardExecutor<T>
where
    T: crate::transport::CardTransport,
{
    type Transport = T;

    fn transport(&self) -> &T {
        &self.transport
    }

    fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Error> {
        // If we can parse the command, process it through our pipeline
        if let Ok(command_obj) = Command::from_bytes(command) {
            // Process the command through our pipeline
            // We need to bypass the clone, so use a reference to our transport
            let mut adapter = crate::processor::TransportAdapter::new(&mut self.transport);
            let result = self
                .pipeline
                .process_command_with_adapter(&command_obj, &mut adapter)
                .context("Error in processor pipeline");

            // Handle the result
            match result {
                Ok(response) => {
                    let response_bytes: Bytes = response.into();
                    self.last_response = Some(response_bytes.clone());
                    return Ok(response_bytes);
                }
                Err(err) => return Err(err),
            }
        }

        // If parsing failed, use raw transport directly
        let result = self
            .transport
            .transmit_raw(command)
            .context("Transport error");
        match result {
            Ok(response) => {
                self.last_response = Some(response.clone());
                Ok(response)
            }
            Err(err) => Err(err),
        }
    }

    fn reset(&mut self) -> Result<(), Error> {
        // Reset the transport
        self.transport
            .reset()
            .context("Failed to reset transport")?;

        // Clear pipeline
        self.pipeline.clear();

        // Clear last response
        self.last_response = None;

        Ok(())
    }

    fn execute<C>(&mut self, command: &C) -> Result<C::Success, C::Error>
    where
        C: ApduCommand,
    {
        // Execute normally - send command bytes and parse response
        let command_bytes = command.to_bytes();
        let response_bytes = self
            .transmit_raw(&command_bytes)
            .map_err(C::convert_error)?;
        let response = Response::from_bytes(&response_bytes)
            .map_err(|e| C::convert_error(e.with_context("Failed to parse response bytes")))?;

        // Parse the response using the command's parse_response method
        C::parse_response(response)
    }
}

// Implementation for CardExecutor with any type parameters for ResponseAwareExecutor
impl<T> ResponseAwareExecutor for CardExecutor<T>
where
    T: crate::transport::CardTransport,
{
    fn last_response(&self) -> Result<&Bytes, Error> {
        self.last_response_bytes()
            .ok_or_else(|| Error::message("No last response available".to_string()))
    }
}

// Extension methods for CardExecutor when transport is a SecureChannel
impl<S> CardExecutor<S>
where
    S: SecureChannel,
{
    /// Get a reference to the secure channel transport
    pub const fn secure_channel(&self) -> &S {
        &self.transport
    }

    /// Get a mutable reference to the secure channel transport
    pub const fn secure_channel_mut(&mut self) -> &mut S {
        &mut self.transport
    }

    /// Set or replace the secure channel transport
    pub fn set_secure_channel(&mut self, secure_channel: S) -> Result<(), Error> {
        self.transport = secure_channel;
        Ok(())
    }
    
    /// Execute a command with automatic security level checking
    ///
    /// This overrides the base execute method to automatically handle secure channel
    /// requirements when the transport is a SecureChannel. Users don't need to call
    /// execute_secure explicitly - the execute method will handle security automatically.
    pub fn execute<C>(&mut self, command: &C) -> Result<C::Success, C::Error>
    where
        C: ApduCommand,
    {
        // Always use the secure execution path for all commands, regardless of
        // whether they explicitly require security or not. This ensures that
        // all commands benefit from the secure channel protection.
        <Self as SecureChannelExecutor>::execute_secure(self, command)
    }

    // (removed unused execute_without_security method)
}

// Implement SecureChannelExecutor for CardExecutor when transport is a SecureChannel
impl<S> SecureChannelExecutor for CardExecutor<S>
where
    S: SecureChannel,
{
    fn has_secure_channel(&self) -> bool {
        self.transport.is_established()
    }

    fn open_secure_channel(&mut self) -> Result<(), Error> {
        self.transport
            .open()
            .context("Failed to establish secure channel")
    }

    fn close_secure_channel(&mut self) -> Result<(), Error> {
        self.transport
            .close()
            .context("Failed to close secure channel")
    }

    fn security_level(&self) -> SecurityLevel {
        self.transport.security_level()
    }
    
    fn upgrade_secure_channel(&mut self, level: SecurityLevel) -> Result<(), Error> {
        self.transport
            .upgrade(level)
            .context("Failed to upgrade secure channel")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::processors::IdentityProcessor;
    use crate::transport::MockTransport;

    #[test]
    fn test_executor_basic_transmit() {
        let transport = MockTransport::with_response(Bytes::from_static(&[0x90, 0x00]));
        let mut executor = CardExecutor::new(transport);

        let response = executor.transmit_raw(&[0x00, 0xA4, 0x04, 0x00]).unwrap();
        assert_eq!(response.as_ref(), &[0x90, 0x00]);
    }

    #[test]
    fn test_executor_with_processor() {
        let transport = MockTransport::with_response(Bytes::from_static(&[0x90, 0x00]));
        let mut executor = CardExecutor::new(transport);

        // Add an identity processor
        executor.add_processor(Box::new(IdentityProcessor));

        let response = executor.transmit_raw(&[0x00, 0xA4, 0x04, 0x00]).unwrap();
        assert_eq!(response.as_ref(), &[0x90, 0x00]);
    }
}
