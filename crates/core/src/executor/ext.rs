//! Extension traits for APDU executors
//!
//! This module provides additional traits that extend the core Executor trait
//! with functionality needed for specialized operations.

use crate::{
    Bytes,
    executor::Executor,
    processor::{SecureProtocolError, secure::SecureChannelProvider},
    transport::{CardTransport, error::TransportError},
};

use super::{ApduExecutorErrors, CardExecutor};

/// Extension trait for executors that support access to the last response
pub trait ResponseAwareExecutor: Executor {
    /// Get the last response received
    ///
    /// Returns the raw bytes of the last response received from the card.
    /// This is useful for protocols that need to access the raw response
    /// for cryptographic operations.
    fn last_response(&self) -> Result<&Bytes, TransportError>;
}

/// Extension trait for executors that support secure channels
pub trait SecureChannelExecutor: Executor {
    /// Open a secure channel with the card
    ///
    /// This establishes a secure channel using the provided secure channel provider.
    fn open_secure_channel(
        &mut self,
        provider: &dyn SecureChannelProvider,
    ) -> Result<(), SecureProtocolError>;

    /// Check if a secure channel is currently established
    fn has_secure_channel(&self) -> bool {
        self.security_level().has_mac_protection() || self.security_level().is_encrypted()
    }
}

// Implementation for CardExecutor with any error type
impl<T: CardTransport, E: ApduExecutorErrors> ResponseAwareExecutor for CardExecutor<T, E> {
    fn last_response(&self) -> Result<&Bytes, TransportError> {
        self.last_response()
            .ok_or_else(|| TransportError::Other("No last response available".to_string()))
    }
}

impl<T: CardTransport, E: ApduExecutorErrors> SecureChannelExecutor for CardExecutor<T, E> {
    fn open_secure_channel(
        &mut self,
        provider: &dyn SecureChannelProvider,
    ) -> Result<(), SecureProtocolError> {
        self.open_secure_channel(provider)
    }
}
