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
pub mod card;
pub mod command;
pub mod error;
pub mod executor;
pub mod processor;
pub mod response;
pub mod secure_channel;
pub mod transport;

pub use command::{ApduCommand, Command, ExpectedLength};
pub use error::{Error, ResultExt};
pub use executor::Executor;
pub use executor::response_aware::ResponseAwareExecutor;
pub use processor::CommandProcessor;
pub use processor::pipeline::ProcessorPipeline;
pub use response::status::StatusWord;
pub use response::{ApduResponse, Response};
pub use secure_channel::{SecureChannel, SecurityLevel};
pub use transport::CardTransport;

/// Prelude module containing commonly used traits and types
pub mod prelude {
    // Core types
    pub use crate::{Bytes, BytesMut, Error, ResultExt};

    // Command related
    pub use crate::Command;
    pub use crate::command::{ApduCommand, ExpectedLength};

    // Response related
    pub use crate::Response;
    pub use crate::response::ApduResponse;
    pub use crate::response::status::{StatusWord, common as status};
    pub use crate::response::utils;

    // Transport layer
    pub use crate::CardTransport;

    // Processor layer
    pub use crate::processor::CommandProcessor;
    pub use crate::processor::pipeline::ProcessorPipeline;
    pub use crate::processor::processors::{GetResponseProcessor, IdentityProcessor};

    // Secure channel layer
    pub use crate::secure_channel::{SecureChannel, SecurityLevel};

    // Executor layer
    pub use crate::executor::Executor;
    pub use crate::executor::SecureChannelExecutor;
    pub use crate::executor::response_aware::ResponseAwareExecutor;

    pub use crate::card::CardExecutor;
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

        let data = Bytes::from_static(&[0x01, 0x02, 0x03]);
        let resp = Response::success(Some(data.clone()));
        assert!(resp.is_success());
        assert_eq!(resp.payload(), &Some(data));
        assert_eq!(resp.status(), StatusWord::new(0x90, 0x00));
    }
}
