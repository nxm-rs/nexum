// apdu-rs/crates/pcsc/src/event/mod.rs
//! Event types and handling for PC/SC operations

pub mod callback;
pub use callback::*;

pub mod channel;
pub use channel::*;

pub mod handler;
pub use handler::*;

/// Events related to card insertion/removal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardEvent {
    /// Card was inserted into a reader
    Inserted {
        /// Reader name
        reader: String,
        /// ATR of the inserted card
        atr: Vec<u8>,
    },
    /// Card was removed from a reader
    Removed {
        /// Reader name
        reader: String,
    },
}

/// Events related to reader connection/disconnection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderEvent {
    /// Reader was connected to the system
    Added(String),
    /// Reader was disconnected from the system
    Removed(String),
}

/// Events related to card status changes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardStatusEvent {
    /// Card state changed
    StateChanged {
        /// Reader name
        reader: String,
        /// New state
        state: CardState,
    },
}

/// Card states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    /// Card is present but not powered
    Present,
    /// Card is unpowered
    Unpowered,
    /// Card is muted (non-responsive)
    Mute,
}
