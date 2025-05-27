//! Extension traits for APDU executors
//!
//! This module provides additional traits that extend the core Executor trait
//! with functionality needed for specialized operations.

use crate::Bytes;
use crate::error::Error;
use crate::executor::Executor;

/// Extension trait for executors that support access to the last response
pub trait ResponseAwareExecutor: Executor {
    /// Get the last response received
    ///
    /// Returns the raw bytes of the last response received from the card.
    /// This is useful for protocols that need to access the raw response
    /// for cryptographic operations.
    fn last_response(&self) -> Result<&Bytes, Error>;
}