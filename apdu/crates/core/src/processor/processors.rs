//! Standard command processors
//!
//! This module provides standard command processors for common protocols.

use std::fmt;

use super::{CommandProcessor, TransportAdapterTrait};
use crate::{ApduCommand, Command, Error, Response, error::ResultExt};

/// Identity processor
///
/// This processor passes commands through unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct IdentityProcessor;

impl CommandProcessor for IdentityProcessor {
    fn process_command_with_adapter(
        &self,
        command: &Command,
        adapter: &mut dyn TransportAdapterTrait,
    ) -> Result<Response, Error> {
        let bytes = command.to_bytes();
        let response_bytes = adapter
            .transmit_raw(&bytes)
            .context("Failed to transmit command")?;
        Response::from_bytes(&response_bytes).context("Failed to parse response")
    }
}

/// GET RESPONSE command processor
///
/// This processor automatically handles status codes that indicate
/// more data is available (61xx) by sending GET RESPONSE commands
/// to retrieve the rest of the data.
#[derive(Clone, Copy)]
pub struct GetResponseProcessor {
    /// Maximum number of chained responses to handle
    pub max_chain: usize,
    /// Class byte for GET RESPONSE command
    pub cla: u8,
}

impl Default for GetResponseProcessor {
    fn default() -> Self {
        Self {
            max_chain: 10, // reasonable default
            cla: 0x00,     // default CLA for GET RESPONSE
        }
    }
}

impl fmt::Debug for GetResponseProcessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GetResponseProcessor")
            .field("max_chain", &self.max_chain)
            .field("cla", &format!("{:#04x}", self.cla))
            .finish()
    }
}

impl CommandProcessor for GetResponseProcessor {
    fn process_command_with_adapter(
        &self,
        command: &Command,
        adapter: &mut dyn TransportAdapterTrait,
    ) -> Result<Response, Error> {
        use bytes::BytesMut;

        let bytes = command.to_bytes();
        let response_bytes = adapter
            .transmit_raw(&bytes)
            .context("Failed to transmit command")?;
        let mut response =
            Response::from_bytes(&response_bytes).context("Failed to parse response")?;

        // Track how many get_response commands we send
        let mut chain_count = 0;

        // Check if we need to send GET RESPONSE
        while response.more_data_available() && chain_count < self.max_chain {
            // Get expected length
            let le = response.bytes_available().unwrap_or(0);

            // Create GET RESPONSE command
            let get_response = Command::new_with_le(self.cla, 0xC0, 0x00, 0x00, le);
            let bytes = get_response.to_bytes();

            // Send command
            let response_bytes = adapter
                .transmit_raw(&bytes)
                .context("Failed to transmit GET RESPONSE command")?;
            let next_response = Response::from_bytes(&response_bytes)
                .context("Failed to parse GET RESPONSE response")?;

            // If we have data, append it to our accumulated data
            if let Some(next_data) = next_response.data.as_ref() {
                let mut buffer = BytesMut::new();

                // Add existing data if any
                if let Some(existing_data) = response.data.as_ref() {
                    buffer.extend_from_slice(existing_data);
                }

                // Add new data
                buffer.extend_from_slice(next_data);

                // Update response
                response.data = Some(buffer.freeze());
            }

            // Update response status
            response.status = next_response.status;

            // Increment chain counter
            chain_count += 1;
        }

        // Check if we hit the chain limit
        if response.more_data_available() {
            return Err(Error::ChainLimitExceeded);
        }

        Ok(response)
    }
}
