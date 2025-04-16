//! Transport layer for card communication
//!
//! This module provides traits for card communication at the transport level.

use bytes::Bytes;
use std::fmt;

use crate::Error;

/// Trait for card transport connections
///
/// Implementors must provide methods for raw transmit and reset.
pub trait CardTransport: fmt::Debug + Send + Sync {
    /// Send a raw APDU command and get the response
    fn transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, Error>;

    /// Reset the transport
    fn reset(&mut self) -> Result<(), Error>;
}

#[cfg(test)]
pub(crate) use mock::MockTransport;

#[cfg(test)]
mod mock {
    use std::sync::Mutex;

    use super::*;

    /// Mock transport for testing
    #[derive(Debug)]
    pub(crate) struct MockTransport {
        /// Response bytes to return
        pub response: Mutex<Bytes>,
    }

    impl MockTransport {
        /// Create a new mock transport with a fixed response
        pub(crate) fn with_response(response: Bytes) -> Self {
            Self {
                response: Mutex::new(response),
            }
        }
    }

    impl CardTransport for MockTransport {
        fn transmit_raw(&mut self, _command: &[u8]) -> Result<Bytes, Error> {
            match self.response.lock() {
                Ok(response) => Ok(response.clone()),
                Err(_) => Err(Error::message(
                    "Failed to lock response in MockTransport".to_string(),
                )),
            }
        }

        fn reset(&mut self) -> Result<(), Error> {
            Ok(())
        }
    }
}
