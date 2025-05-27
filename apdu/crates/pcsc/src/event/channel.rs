//! Channel-based event handling for PC/SC operations

use crate::event::{CardEvent, CardStatusEvent, ReaderEvent};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

/// Sender for card events
pub type CardEventSender = Sender<CardEvent>;
/// Receiver for card events
pub type CardEventReceiver = Receiver<CardEvent>;

/// Sender for reader events
pub type ReaderEventSender = Sender<ReaderEvent>;
/// Receiver for reader events
pub type ReaderEventReceiver = Receiver<ReaderEvent>;

/// Sender for card status events
pub type CardStatusEventSender = Sender<CardStatusEvent>;
/// Receiver for card status events
pub type CardStatusEventReceiver = Receiver<CardStatusEvent>;

/// Create an unbounded channel for card events
pub fn card_event_channel() -> (CardEventSender, CardEventReceiver) {
    unbounded()
}

/// Create an unbounded channel for reader events
pub fn reader_event_channel() -> (ReaderEventSender, ReaderEventReceiver) {
    unbounded()
}

/// Create an unbounded channel for card status events
pub fn card_status_event_channel() -> (CardStatusEventSender, CardStatusEventReceiver) {
    unbounded()
}

/// Create a bounded channel with the specified capacity for card events
pub fn bounded_card_event_channel(capacity: usize) -> (CardEventSender, CardEventReceiver) {
    bounded(capacity)
}

/// Create a bounded channel with the specified capacity for reader events
pub fn bounded_reader_event_channel(capacity: usize) -> (ReaderEventSender, ReaderEventReceiver) {
    bounded(capacity)
}

/// Create a bounded channel with the specified capacity for card status events
pub fn bounded_card_status_event_channel(
    capacity: usize,
) -> (CardStatusEventSender, CardStatusEventReceiver) {
    bounded(capacity)
}
