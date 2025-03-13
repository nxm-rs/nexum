//! Channel-based event handling for PC/SC operations

/// Standard library implementation
#[cfg(feature = "std")]
pub mod std_channel {
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
    pub fn bounded_reader_event_channel(
        capacity: usize,
    ) -> (ReaderEventSender, ReaderEventReceiver) {
        bounded(capacity)
    }

    /// Create a bounded channel with the specified capacity for card status events
    pub fn bounded_card_status_event_channel(
        capacity: usize,
    ) -> (CardStatusEventSender, CardStatusEventReceiver) {
        bounded(capacity)
    }
}

// No-std implementation using heapless
#[cfg(feature = "alloc")]
pub mod no_std_channel {
    use crate::event::{CardEvent, CardStatusEvent, ReaderEvent};
    use alloc::boxed::Box;
    use alloc::sync::Arc;
    use core::cell::RefCell;
    use core::sync::atomic::{AtomicBool, Ordering};
    use heapless::spsc::{Consumer, Producer, Queue};

    /// Producer for card events with fixed capacity N
    pub struct CardEventProducer<const N: usize> {
        queue: RefCell<Producer<'static, CardEvent, N>>,
    }

    /// Consumer for card events with fixed capacity N
    pub struct CardEventConsumer<const N: usize> {
        queue: RefCell<Consumer<'static, CardEvent, N>>,
    }

    /// Producer for reader events with fixed capacity N
    pub struct ReaderEventProducer<const N: usize> {
        queue: RefCell<Producer<'static, ReaderEvent, N>>,
    }

    /// Consumer for reader events with fixed capacity N
    pub struct ReaderEventConsumer<const N: usize> {
        queue: RefCell<Consumer<'static, ReaderEvent, N>>,
    }

    /// Producer for card status events with fixed capacity N
    pub struct CardStatusEventProducer<const N: usize> {
        queue: RefCell<Producer<'static, CardStatusEvent, N>>,
    }

    /// Consumer for card status events with fixed capacity N
    pub struct CardStatusEventConsumer<const N: usize> {
        queue: RefCell<Consumer<'static, CardStatusEvent, N>>,
    }

    impl<const N: usize> CardEventProducer<N> {
        /// Try to send an event, returns false if queue is full
        pub fn try_send(&self, event: CardEvent) -> bool {
            match self.queue.borrow_mut().enqueue(event) {
                Ok(()) => true,
                Err(_) => false,
            }
        }
    }

    impl<const N: usize> CardEventConsumer<N> {
        /// Try to receive an event, returns None if queue is empty
        pub fn try_recv(&self) -> Option<CardEvent> {
            self.queue.borrow_mut().dequeue()
        }
    }

    impl<const N: usize> ReaderEventProducer<N> {
        /// Try to send an event, returns false if queue is full
        pub fn try_send(&self, event: ReaderEvent) -> bool {
            match self.queue.borrow_mut().enqueue(event) {
                Ok(()) => true,
                Err(_) => false,
            }
        }
    }

    impl<const N: usize> ReaderEventConsumer<N> {
        /// Try to receive an event, returns None if queue is empty
        pub fn try_recv(&self) -> Option<ReaderEvent> {
            self.queue.borrow_mut().dequeue()
        }
    }

    impl<const N: usize> CardStatusEventProducer<N> {
        /// Try to send an event, returns false if queue is full
        pub fn try_send(&self, event: CardStatusEvent) -> bool {
            match self.queue.borrow_mut().enqueue(event) {
                Ok(()) => true,
                Err(_) => false,
            }
        }
    }

    impl<const N: usize> CardStatusEventConsumer<N> {
        /// Try to receive an event, returns None if queue is empty
        pub fn try_recv(&self) -> Option<CardStatusEvent> {
            self.queue.borrow_mut().dequeue()
        }
    }

    /// Create a channel for card events with capacity N
    pub fn card_event_channel<const N: usize>() -> (CardEventProducer<N>, CardEventConsumer<N>) {
        // Create a static queue using a heap allocation (safe in no_std+alloc)
        let queue: Box<Queue<CardEvent, N>> = Box::new(Queue::new());
        let queue_ref = Box::leak(queue);

        let (producer, consumer) = queue_ref.split();

        (
            CardEventProducer {
                queue: RefCell::new(producer),
            },
            CardEventConsumer {
                queue: RefCell::new(consumer),
            },
        )
    }

    /// Create a channel for reader events with capacity N
    pub fn reader_event_channel<const N: usize>() -> (ReaderEventProducer<N>, ReaderEventConsumer<N>)
    {
        // Create a static queue using a heap allocation (safe in no_std+alloc)
        let queue: Box<Queue<ReaderEvent, N>> = Box::new(Queue::new());
        let queue_ref = Box::leak(queue);

        let (producer, consumer) = queue_ref.split();

        (
            ReaderEventProducer {
                queue: RefCell::new(producer),
            },
            ReaderEventConsumer {
                queue: RefCell::new(consumer),
            },
        )
    }

    /// Create a channel for card status events with capacity N
    pub fn card_status_event_channel<const N: usize>()
    -> (CardStatusEventProducer<N>, CardStatusEventConsumer<N>) {
        // Create a static queue using a heap allocation (safe in no_std+alloc)
        let queue: Box<Queue<CardStatusEvent, N>> = Box::new(Queue::new());
        let queue_ref = Box::leak(queue);

        let (producer, consumer) = queue_ref.split();

        (
            CardStatusEventProducer {
                queue: RefCell::new(producer),
            },
            CardStatusEventConsumer {
                queue: RefCell::new(consumer),
            },
        )
    }
}

// Re-export based on features
#[cfg(feature = "std")]
pub use std_channel::*;

#[cfg(feature = "alloc")]
pub use no_std_channel::*;
