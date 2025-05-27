//! Callback-based event handling for PC/SC operations

use crate::event::{CardEvent, CardStatusEvent, ReaderEvent};

/// Trait for handling card events
pub trait CardEventHandler {
    /// Handle a card event
    fn handle_event(&mut self, event: CardEvent);
}

/// Trait for handling reader events
pub trait ReaderEventHandler {
    /// Handle a reader event
    fn handle_event(&mut self, event: ReaderEvent);
}

/// Trait for handling card status events
pub trait CardStatusEventHandler {
    /// Handle a card status event
    fn handle_event(&mut self, event: CardStatusEvent);
}

// Implement handlers for closures
impl<F> CardEventHandler for F
where
    F: FnMut(CardEvent),
{
    fn handle_event(&mut self, event: CardEvent) {
        self(event)
    }
}

impl<F> ReaderEventHandler for F
where
    F: FnMut(ReaderEvent),
{
    fn handle_event(&mut self, event: ReaderEvent) {
        self(event)
    }
}

impl<F> CardStatusEventHandler for F
where
    F: FnMut(CardStatusEvent),
{
    fn handle_event(&mut self, event: CardStatusEvent) {
        self(event)
    }
}
