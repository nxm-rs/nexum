//! PC/SC transport implementation for APDU operations
//!
//! This crate provides an implementation of the `CardTransport` trait from
//! `apdu-core` using the PC/SC API for communication with smart cards.
//!
//! # Features
//!
//! - `std` (default): Use standard library features and PC/SC system libraries
//! - `alloc`: Support for no_std environments with allocator
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "std")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use apdu_core::{CardExecutor, Command};
//! use apdu_core::prelude::Executor;
//! use apdu_transport_pcsc::{PcscDeviceManager, PcscConfig};
//!
//! // Create a PC/SC device manager
//! let manager = PcscDeviceManager::new()?;
//!
//! // List available readers
//! let readers = manager.list_readers()?;
//! if readers.is_empty() {
//!     println!("No readers found");
//!     return Ok(());
//! }
//!
//! // Connect to the first reader
//! let reader = &readers[0];
//! println!("Connecting to reader: {}", reader.name());
//!
//! let transport = manager.open_reader(reader.name())?;
//! let mut executor = CardExecutor::new(transport);
//!
//! // Send a SELECT command
//! let aid = hex::decode("A000000003000000").unwrap();
//! let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid);
//!
//! match executor.execute(&select_cmd) {
//!     Ok(response) => {
//!         println!("Response: {:?}", response);
//!     }
//!     Err(e) => {
//!         println!("Error: {:?}", e);
//!     }
//! }
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "std"))]
//! # fn main() {}
//! ```
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

// Core modules
mod config;
mod error;
pub mod event;
mod manager;
mod monitor;
mod reader;
mod transport;
mod util;

// Public exports
pub use config::{ConnectStrategy, PcscConfig, ShareMode, TransactionMode};
pub use error::PcscError;
pub use event::{CardEvent, ReaderEvent};
pub use manager::PcscDeviceManager;
pub use monitor::PcscMonitor;
pub use reader::PcscReader;
pub use transport::PcscTransport;

// Re-export some pcsc types for convenience
#[cfg(feature = "std")]
pub use pcsc::{Protocol, Protocols, Status};
