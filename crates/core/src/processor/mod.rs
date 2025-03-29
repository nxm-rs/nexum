//! Command processors for APDU transformations
//!
//! This module provides abstractions for processing APDU commands before
//! sending them to a card transport. Command processors can implement various
//! transformations such as secure messaging, logging, or retry logic.

pub mod error;
pub mod secure;

use bytes::Bytes;
use core::fmt;
use dyn_clone::DynClone;
use secure::SecurityLevel;
use tracing::{debug, trace};

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use crate::command::Command;
use crate::command::ExpectedLength;
use crate::response::Response;
use crate::response::utils;
use crate::transport::CardTransport;
use crate::{ApduCommand, ApduResponse};
use error::ProcessorError;

/// Trait for command processors which transform commands
/// before sending them to the transport
pub trait CommandProcessor: Send + Sync + fmt::Debug + DynClone {
    /// Process a command through this processor
    ///
    /// This method takes a command, potentially transforms it, sends it through
    /// the transport, and potentially transforms the response.
    fn process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError> {
        trace!(
            command = ?command,
            processor = std::any::type_name::<Self>(),
            "Processing command"
        );

        let result = self.do_process_command(command, transport);

        match &result {
            Ok(response) => {
                trace!(
                    response = ?response,
                    "Processed response"
                );
            }
            Err(e) => {
                debug!(
                    error = ?e,
                    "Error during command processing"
                );
            }
        }

        result
    }

    /// Internal implementation of process_command
    fn do_process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError>;

    /// Get the security level provided by this processor
    fn security_level(&self) -> SecurityLevel {
        SecurityLevel::NoSecurity
    }

    /// Check if this processor is active/ready
    fn is_active(&self) -> bool {
        true
    }
}

// Enable cloning for boxed processors
dyn_clone::clone_trait_object!(CommandProcessor);

/// Identity processor that doesn't modify commands
#[derive(Debug, Clone)]
pub struct IdentityProcessor;

impl CommandProcessor for IdentityProcessor {
    fn do_process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError> {
        // Convert command to bytes and send
        let command_bytes = command.to_bytes();
        let response_bytes = transport
            .transmit_raw(&command_bytes)
            .map_err(ProcessorError::from)?;

        // Parse response
        Response::from_bytes(&response_bytes)
            .map_err(|_| ProcessorError::InvalidResponse("Failed to parse response"))
    }
}

/// GET RESPONSE processor that handles automatic response chaining
#[derive(Debug, Clone)]
pub struct GetResponseProcessor {
    /// Maximum number of response chains to follow
    max_chains: usize,
}

impl GetResponseProcessor {
    /// Create a new GET RESPONSE processor with the given maximum chain count
    pub const fn new(max_chains: usize) -> Self {
        Self { max_chains }
    }
}

impl Default for GetResponseProcessor {
    fn default() -> Self {
        Self::new(10) // Reasonable default
    }
}

impl CommandProcessor for GetResponseProcessor {
    fn do_process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError> {
        // Convert command to bytes
        let command_bytes = command.to_bytes();

        // First send the original command
        let response_bytes = transport
            .transmit_raw(&command_bytes)
            .map_err(ProcessorError::from)?;

        // Extract status and payload
        let ((sw1, sw2), payload) = utils::extract_response_parts(&response_bytes)
            .map_err(|_| ProcessorError::InvalidResponse("Response too short"))?;

        // If SW1=61, use GET RESPONSE to fetch more data
        if sw1 == 0x61 {
            let mut buffer = Vec::new();

            // Save any payload data from initial response
            buffer.extend_from_slice(payload);

            let mut chains = 0;
            let mut current_sw1 = sw1;
            let mut current_sw2 = sw2;

            // Process GET RESPONSE chain
            while current_sw1 == 0x61 && chains < self.max_chains {
                // Build GET RESPONSE command
                let get_response =
                    Command::new(0x00, 0xC0, 0x00, 0x00).with_le(current_sw2 as ExpectedLength);
                let get_resp_bytes = get_response.to_bytes();

                trace!(
                    remaining = current_sw2,
                    chain_count = chains + 1,
                    "Sending GET RESPONSE command"
                );

                // Send GET RESPONSE
                let response_bytes = transport
                    .transmit_raw(&get_resp_bytes)
                    .map_err(ProcessorError::from)?;

                // Extract status and payload from response
                let ((new_sw1, new_sw2), new_payload) =
                    utils::extract_response_parts(&response_bytes).map_err(|_| {
                        ProcessorError::InvalidResponse("GET RESPONSE returned incomplete data")
                    })?;

                // Add payload to buffer
                buffer.extend_from_slice(new_payload);

                // Update status for potential next iteration
                current_sw1 = new_sw1;
                current_sw2 = new_sw2;
                chains += 1;
            }

            if chains >= self.max_chains && current_sw1 == 0x61 {
                return Err(ProcessorError::ChainLimitExceeded);
            }

            // Construct final response with accumulated data and final status word
            let response = Response::new(Bytes::from(buffer), (current_sw1, current_sw2));

            trace!(
                total_data_len = response.payload().len(),
                final_sw = format!("{:02X}{:02X}", current_sw1, current_sw2),
                "Completed response chaining"
            );

            return Ok(response);
        }

        // If no chaining needed, create response directly
        Ok(Response::new(Bytes::copy_from_slice(payload), (sw1, sw2)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ApduCommand, ApduResponse, transport::MockTransport};

    #[test]
    fn test_identity_processor() {
        let mut transport = MockTransport::with_response(Bytes::from_static(&[0x90, 0x00]));
        let mut processor = IdentityProcessor;

        let command = Command::new(0x00, 0xA4, 0x04, 0x00);
        let response = processor.process_command(&command, &mut transport).unwrap();

        assert_eq!(response.status().to_u16(), 0x9000);
        assert_eq!(transport.commands[0], command.to_bytes());
    }

    #[test]
    fn test_get_response_processor() {
        let mut transport = MockTransport::new(Vec::new());

        // First response: 61 05 (more data available)
        transport.responses.push(Bytes::from_static(&[0x61, 0x05]));

        // GET RESPONSE response: data + final status
        transport.responses.push(Bytes::from_static(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x90, 0x00,
        ]));

        let mut processor = GetResponseProcessor::default();

        let command = Command::new(0x00, 0xB0, 0x00, 0x00);
        let response = processor.process_command(&command, &mut transport).unwrap();

        // Should have the combined data with final status
        assert_eq!(response.payload(), &[0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(response.status().to_u16(), 0x9000);

        // Should have sent the original command and the GET RESPONSE command
        assert_eq!(transport.commands[0], command.to_bytes());
        let get_resp_cmd = Command::new(0x00, 0xC0, 0x00, 0x00).with_le(5);
        assert_eq!(transport.commands[1], get_resp_cmd.to_bytes());
    }
}
