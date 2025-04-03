//! Event handler traits and utilities

use crate::event::{CardEvent, CardStatusEvent, ReaderEvent};

/// A trait for types that can be used as event handlers
pub trait EventHandler<T> {
    /// Handle an event
    fn handle(&mut self, event: T);
}

/// A trait for types that can be converted into event handlers
pub trait IntoEventHandler<T> {
    /// The handler type
    type Handler: EventHandler<T>;

    /// Convert to handler
    fn into_handler(self) -> Self::Handler;
}

// Implementations for closures
impl<T, F> EventHandler<T> for F
where
    F: FnMut(T),
{
    fn handle(&mut self, event: T) {
        self(event)
    }
}

impl<T, F> IntoEventHandler<T> for F
where
    F: FnMut(T) + 'static,
{
    type Handler = F;

    fn into_handler(self) -> Self::Handler {
        self
    }
}

/// Simple event dispatcher that manages multiple handlers
#[allow(missing_debug_implementations)]
pub struct EventDispatcher<T> {
    /// Collection of event handlers
    handlers: Vec<Box<dyn EventHandler<T>>>,
}

impl<T> EventDispatcher<T> {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Add a new handler
    pub fn add_handler<H>(&mut self, handler: H)
    where
        H: EventHandler<T> + 'static,
    {
        self.handlers.push(Box::new(handler));
    }

    /// Dispatch an event to all handlers
    pub fn dispatch(&mut self, event: T)
    where
        T: Clone,
    {
        for handler in &mut self.handlers {
            handler.handle(event.clone());
        }
    }

    /// Clear all handlers
    pub fn clear(&mut self) {
        self.handlers.clear();
    }
}

impl<T> Default for EventDispatcher<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Dispatcher for card events
pub type CardEventDispatcher = EventDispatcher<CardEvent>;

/// Dispatcher for reader events
pub type ReaderEventDispatcher = EventDispatcher<ReaderEvent>;

/// Dispatcher for card status events
pub type CardStatusEventDispatcher = EventDispatcher<CardStatusEvent>;
