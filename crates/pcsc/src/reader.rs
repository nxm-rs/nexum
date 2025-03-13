//! Reader representation for PC/SC devices

#[cfg(feature = "std")]
use pcsc::{ReaderState, State};

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::string::String;
#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::vec::Vec;

/// Representation of a PC/SC card reader
#[derive(Debug, Clone)]
pub struct PcscReader {
    /// Name of the reader
    #[cfg(any(feature = "std", feature = "alloc"))]
    name: String,

    /// Whether a card is present
    has_card: bool,

    /// Answer To Reset of the card (if present)
    #[cfg(any(feature = "std", feature = "alloc"))]
    atr: Option<Vec<u8>>,
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl PcscReader {
    /// Create a new reader
    pub const fn new(name: String, has_card: bool, atr: Option<Vec<u8>>) -> Self {
        Self {
            name,
            has_card,
            atr,
        }
    }

    /// Get the reader name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if a card is present in the reader
    pub const fn has_card(&self) -> bool {
        self.has_card
    }

    /// Get the ATR of the card if present
    pub fn atr(&self) -> Option<&[u8]> {
        self.atr.as_deref()
    }
}

#[cfg(feature = "std")]
impl PcscReader {
    /// Create a reader from a reader state
    pub(crate) fn from_reader_state(reader_state: &ReaderState) -> Self {
        let has_card = reader_state.event_state().contains(State::PRESENT)
            && !reader_state.event_state().contains(State::EMPTY);

        let atr = if has_card {
            Some(reader_state.atr().to_vec())
        } else {
            None
        };

        Self {
            name: reader_state.name().to_string_lossy().into_owned(),
            has_card,
            atr,
        }
    }
}
