//! Executor for APDU command execution
//!
//! This module provides traits and error types for APDU command execution.
//! The actual card executor implementation is in the `card` module.

pub mod response_aware;

use std::fmt;

use crate::command::{ApduCommand, Command};
use crate::error::Error;
use crate::secure_channel::SecurityLevel;
use crate::{CardTransport, Response};
use bytes::Bytes;
use tracing::{debug, instrument, trace};

// Re-export extension traits
pub use response_aware::ResponseAwareExecutor;

/// Trait for APDU command execution
pub trait Executor: Send + Sync + fmt::Debug {
    /// The transport type used by this executor
    type Transport: CardTransport;

    /// Get a reference to the underlying transport
    fn transport(&self) -> &Self::Transport;

    /// Get a mutable reference to the underlying transport
    fn transport_mut(&mut self) -> &mut Self::Transport;

    /// Transmit a raw APDU command
    ///
    /// This is the lowest level public transmission method.
    #[instrument(level = "trace", skip(self), fields(executor = std::any::type_name::<Self>()))]
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Error> {
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
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Error>;

    /// Transmit a generic Command and return a Response
    ///
    /// This is the mid-level transmission method that works with Command and Response objects.
    fn transmit(&mut self, command: &Command) -> Result<Response, Error> {
        trace!(command = ?command, "Transmitting command");
        let command_bytes = command.to_bytes();
        let response_bytes = self.transmit_raw(&command_bytes)?;
        let response = Response::from_bytes(&response_bytes)
            .map_err(|e| e.with_context("Failed to parse response bytes"))?;
        trace!(response = ?response, "Received response");
        Ok(response)
    }

    /// Execute a typed APDU command and return the command's success type
    ///
    /// This method returns the command's Success type directly for more
    /// idiomatic error handling with the ? operator. The error type is the command's
    /// own error type, allowing commands to define their own error handling.
    fn execute<C>(&mut self, command: &C) -> Result<C::Success, C::Error>
    where
        C: ApduCommand;

    /// Reset the executor, including the transport
    fn reset(&mut self) -> Result<(), Error>;
}

/// Extension trait for executors that support secure channel operations
///
/// This trait extends the base Executor trait with methods specific to secure channel
/// management and execution. It is implemented for executors that have a transport
/// which implements the SecureChannel trait.
pub trait SecureChannelExecutor: Executor {
    /// Check if the executor has an established secure channel
    fn has_secure_channel(&self) -> bool;

    /// Open  the secure channel with the card
    fn open_secure_channel(&mut self) -> Result<(), Error>;

    /// Close an established secure channel
    fn close_secure_channel(&mut self) -> Result<(), Error>;

    /// Get current security level of the secure channel
    fn security_level(&self) -> SecurityLevel;

    /// Upgrade the secure channel to the specified security level
    fn upgrade_secure_channel(&mut self, level: SecurityLevel) -> Result<(), Error>;

    /// Execute a command with security level checking
    ///
    /// This method checks if the command requires a certain security level,
    /// attempts to upgrade the secure channel if necessary, and then executes
    /// the command. If the command doesn't require a secure channel or if the
    /// security level is already sufficient, it will execute the command normally.
    fn execute_secure<C>(&mut self, command: &C) -> Result<C::Success, C::Error>
    where
        C: ApduCommand,
    {
        // Check security level requirement
        let required_level = command.required_security_level();

        // If no security required, execute the command as-is without additional security
        // but still use the execute_direct method to avoid recursion
        if required_level.is_none() {
            // Use a low-level approach to execute the command directly
            let command_bytes = command.to_bytes();
            let response_bytes = self.transmit_raw(&command_bytes).map_err(C::convert_error)?;
            let response = Response::from_bytes(&response_bytes)
                .map_err(|e| C::convert_error(e.with_context("Failed to parse response bytes")))?;
            return C::parse_response(response);
        }

        // Check current security level
        let current_level = self.security_level();

        // If security level is insufficient, try to upgrade the channel
        if !current_level.satisfies(&required_level) {
            // If the secure channel isn't established, try to establish it
            if !self.has_secure_channel() {
                self.open_secure_channel().map_err(C::convert_error)?;
            }

            // Try to upgrade the channel to the required level
            self.upgrade_secure_channel(required_level)
                .map_err(C::convert_error)?;

            // Check if security level is now sufficient
            if !self.security_level().satisfies(&required_level) {
                return Err(C::convert_error(Error::InsufficientSecurityLevel {
                    required: required_level,
                    current: self.security_level(),
                }));
            }
        }

        // Now that security is established, execute the command directly
        // Use a low-level approach to execute the command directly to avoid recursion
        let command_bytes = command.to_bytes();
        let response_bytes = self.transmit_raw(&command_bytes).map_err(C::convert_error)?;
        let response = Response::from_bytes(&response_bytes)
            .map_err(|e| C::convert_error(e.with_context("Failed to parse response bytes")))?;
        C::parse_response(response)
    }
}
