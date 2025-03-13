//! Secure channel implementations for APDU processing
//!
//! This module provides secure channel implementations that work as command processors.

use bytes::Bytes;
use core::fmt;
use dyn_clone::DynClone;
use tracing::{debug, warn};

#[cfg(test)]
use tracing::trace;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use super::{CommandProcessor, error::ProcessorError};
use crate::ApduCommand;
use crate::command::Command;
use crate::response::Response;
use crate::transport::CardTransport;

/// Security level for communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// No security (plain communication)
    NoSecurity,
    /// Authentication only
    Authenticated,
    /// Message Authentication Codes (data integrity)
    MACProtection,
    /// Full encryption (data confidentiality and integrity)
    FullEncryption,
}

/// Trait for secure channel providers
pub trait SecureChannelProvider: Send + Sync + fmt::Debug {
    /// Create a new secure channel with the specified security level
    fn create_secure_channel(
        &self,
        transport: &mut dyn CardTransport,
    ) -> Result<Box<dyn CommandProcessor>, ProcessorError>;
}

/// Generic secure channel base trait with common functionality
///
/// This trait extends CommandProcessor with secure channel specific methods
pub trait SecureChannel: CommandProcessor + DynClone {
    /// Check if the secure channel is established
    fn is_established(&self) -> bool;

    /// Close the secure channel
    fn close(&mut self) -> Result<(), ProcessorError>;

    /// Reestablish a closed channel
    fn reestablish(&mut self) -> Result<(), ProcessorError>;
}

/// A base secure channel implementation that can be extended
#[derive(Debug, Clone)]
pub struct BaseSecureChannel {
    /// Whether the channel is established
    established: bool,
    /// Session data for the channel
    session_data: Option<Bytes>,
}

impl Default for BaseSecureChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseSecureChannel {
    /// Create a new base secure channel
    pub const fn new() -> Self {
        Self {
            established: false,
            session_data: None,
        }
    }

    /// Set the session data
    pub fn set_session_data(&mut self, data: Bytes) {
        self.session_data = Some(data);
    }

    /// Get the session data
    pub const fn session_data(&self) -> Option<&Bytes> {
        self.session_data.as_ref()
    }

    /// Mark the channel as established
    pub fn set_established(&mut self, established: bool) {
        self.established = established;
    }
}

impl CommandProcessor for BaseSecureChannel {
    fn do_process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError> {
        warn!("Using BaseSecureChannel which does not implement any protection");

        let command_bytes = command.to_bytes();
        let response_bytes = transport
            .transmit_raw(&command_bytes)
            .map_err(ProcessorError::from)?;

        Response::from_bytes(&response_bytes)
            .map_err(|_| ProcessorError::InvalidResponse("Failed to parse response"))
    }

    fn is_active(&self) -> bool {
        self.established
    }
}

impl SecureChannel for BaseSecureChannel {
    fn is_established(&self) -> bool {
        self.established
    }

    fn close(&mut self) -> Result<(), ProcessorError> {
        debug!("Closing secure channel");
        self.established = false;
        self.session_data = None;
        Ok(())
    }

    fn reestablish(&mut self) -> Result<(), ProcessorError> {
        warn!("BaseSecureChannel cannot reestablish without proper implementation");
        Err(ProcessorError::session(
            "Cannot reestablish base secure channel",
        ))
    }
}

/// Mock secure channel for testing
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MockSecureChannel {
    base: BaseSecureChannel,
}

#[cfg(test)]
impl MockSecureChannel {
    /// Create a new mock secure channel
    pub fn new() -> Self {
        let mut base = BaseSecureChannel::new();
        base.set_established(true);

        Self { base }
    }
}

#[cfg(test)]
impl CommandProcessor for MockSecureChannel {
    fn do_process_command(
        &mut self,
        command: &Command,
        transport: &mut dyn CardTransport,
    ) -> Result<Response, ProcessorError> {
        if !self.is_established() {
            return Err(ProcessorError::session("Secure channel not established"));
        }

        // Create a secured version of the command
        let secured_cmd = Command::new(
            command.class(),
            command.instruction(),
            command.p1(),
            command.p2(),
        );

        // Add data and Le if present in original command
        let secured_cmd = if let Some(data) = command.data() {
            secured_cmd.with_data(data.to_vec())
        } else {
            secured_cmd
        };

        let secured_cmd = if let Some(le) = command.expected_length() {
            secured_cmd.with_le(le)
        } else {
            secured_cmd
        };

        trace!(
            level = ?self.security_level(),
            "MockSecureChannel processed command"
        );

        // Send the secured command
        let secured_bytes = secured_cmd.to_bytes();
        let response_bytes = transport
            .transmit_raw(&secured_bytes)
            .map_err(ProcessorError::from)?;

        // Parse and return response
        Response::from_bytes(&response_bytes)
            .map_err(|_| ProcessorError::InvalidResponse("Failed to parse response"))
    }

    fn security_level(&self) -> SecurityLevel {
        self.base.security_level()
    }

    fn is_active(&self) -> bool {
        self.base.is_established()
    }
}

#[cfg(test)]
impl SecureChannel for MockSecureChannel {
    fn is_established(&self) -> bool {
        self.base.is_established()
    }

    fn close(&mut self) -> Result<(), ProcessorError> {
        debug!("Closing mock secure channel");
        self.base.set_established(false);
        Ok(())
    }

    fn reestablish(&mut self) -> Result<(), ProcessorError> {
        debug!("Reestablishing mock secure channel");
        self.base.set_established(true);
        Ok(())
    }
}
