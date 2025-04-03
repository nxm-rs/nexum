//! Core traits and types for APDU (Application Protocol Data Unit) operations
//!
//! This crate provides the foundational types and traits for working with smart card
//! APDU commands and responses according to ISO/IEC 7816-4.
//!
//! ## Overview
//!
//! APDU (Application Protocol Data Unit) is the communication format used by smart cards.
//! This crate provides abstractions for:
//!
//! - Creating and parsing APDU commands and responses
//! - Communicating with smart cards through different transport layers
//! - Handling secure communication channels
//! - Error handling and status word interpretation
//!
//! The crate is designed to be flexible and extensible while supporting both std and no_std environments.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![forbid(unsafe_code)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]

// Re-export bytes for convenience
pub use bytes::{Bytes, BytesMut};

// Main modules
pub mod command;
pub mod executor;
pub mod processor;
pub mod response;
pub mod transport;

// Core error types
mod error;
pub use error::{Error, Result};

// Re-exports for common types
pub use command::{ApduCommand, Command};
pub use executor::ext::{ResponseAwareExecutor, SecureChannelExecutor}; // New re-exports
pub use executor::{CardExecutor, Executor};
pub use response::status::StatusWord;
pub use response::{ApduResponse, Response, utils};
pub use transport::CardTransport;

/// Prelude module containing commonly used traits and types
pub mod prelude {
    pub use crate::{
        Bytes, BytesMut, Command, Error, Response, Result,
        command::ApduCommand,
        executor::Executor,
        executor::ext::{ResponseAwareExecutor, SecureChannelExecutor},
        processor::CommandProcessor,
        response::status::StatusWord,
        response::{ApduResponse, FromApduResponse},
        transport::CardTransport,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    // Test the basic types are re-exported correctly
    #[test]
    fn test_reexports() {
        let cmd = Command::new(0x00, 0xA4, 0x04, 0x00);
        assert_eq!(cmd.class(), 0x00);
        assert_eq!(cmd.instruction(), 0xA4);
        assert_eq!(cmd.p1(), 0x04);
        assert_eq!(cmd.p2(), 0x00);

        let resp = Response::success(Bytes::from_static(&[0x01, 0x02, 0x03]));
        assert!(resp.is_success());
        assert_eq!(resp.payload(), &[0x01, 0x02, 0x03]);
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
    }
}
