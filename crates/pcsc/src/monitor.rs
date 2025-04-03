// apdu-rs/crates/pcsc/src/monitor.rs
//! Monitor implementation for PC/SC events

use pcsc::{Context, ReaderState, Scope, State};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::error::PcscError;
use crate::event::callback::{CardEventHandler, CardStatusEventHandler, ReaderEventHandler};
use crate::event::{CardEvent, CardState, CardStatusEvent, ReaderEvent};

use crate::event::channel::{CardEventSender, CardStatusEventSender, ReaderEventSender};

/// Monitor for PC/SC reader and card events
#[allow(missing_debug_implementations)]
pub struct PcscMonitor {
    /// PC/SC context
    context: Context,
    /// Whether the monitor is running
    running: Arc<Mutex<bool>>,
    /// Previously seen card events (to avoid duplicates)
    previous_states: Arc<Mutex<HashMap<String, (State, Vec<u8>)>>>,
}

// Implementation for standard library environments
impl PcscMonitor {
    /// Create a new monitor
    pub(crate) fn new(context: Context) -> Result<Self, PcscError> {
        Ok(Self {
            context,
            running: Arc::new(Mutex::new(false)),
            previous_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create a new monitor with a dedicated context
    pub fn create() -> Result<Self, PcscError> {
        let context = Context::establish(Scope::User)?;
        Self::new(context)
    }

    /// Wait for events with a timeout
    pub fn wait_for_card_events(&mut self, timeout: Duration) -> Result<Vec<CardEvent>, PcscError> {
        // Initialize with the PnP notification
        let mut reader_states = vec![ReaderState::new(pcsc::PNP_NOTIFICATION(), State::UNAWARE)];

        // Get initial readers
        let readers = self.context.list_readers_owned()?;
        for reader in readers {
            reader_states.push(ReaderState::new(reader, State::UNAWARE));
        }

        // Update the current state to wait on
        for rs in &mut reader_states {
            rs.sync_current_state();
        }

        // Wait for state changes
        self.context
            .get_status_change(Some(timeout), &mut reader_states)?;

        let mut events = Vec::new();
        let mut previous_states = self.previous_states.lock().unwrap();

        // Process events
        for rs in &reader_states {
            let name = rs.name().to_string_lossy().into_owned();
            let event_state = rs.event_state();

            // Skip PnP notification
            if name == pcsc::PNP_NOTIFICATION().to_string_lossy() {
                continue;
            }

            // Card inserted
            if event_state.contains(State::PRESENT) && !event_state.contains(State::EMPTY) {
                let atr = rs.atr().to_vec();

                // Check if this is a new insertion or a different card
                let is_new_event = match previous_states.get(&name) {
                    Some((prev_state, prev_atr)) => {
                        !prev_state.contains(State::PRESENT) || *prev_atr != atr
                    }
                    None => true,
                };

                if is_new_event {
                    events.push(CardEvent::Inserted {
                        reader: name.clone(),
                        atr: atr.clone(),
                    });
                    // Update state
                    previous_states.insert(name, (event_state, atr));
                }
            }
            // Card removed
            else if event_state.contains(State::EMPTY) {
                let is_new_event = match previous_states.get(&name) {
                    Some((prev_state, _)) => prev_state.contains(State::PRESENT),
                    None => false, // Don't report removal if we never saw it present
                };

                if is_new_event {
                    events.push(CardEvent::Removed {
                        reader: name.clone(),
                    });
                    // Update state - empty ATR for removed card
                    previous_states.insert(name, (event_state, Vec::new()));
                }
            }
        }

        Ok(events)
    }

    /// Check for reader changes
    pub fn check_reader_changes(&mut self) -> Result<Vec<ReaderEvent>, PcscError> {
        let mut events = Vec::new();
        let mut previous_states = self.previous_states.lock().unwrap();

        // Update readers list
        let current_readers = self.context.list_readers_owned()?;
        let current_names: Vec<String> = current_readers
            .iter()
            .map(|r| r.to_string_lossy().into_owned())
            .collect();

        // Find new readers
        for name in &current_names {
            if !previous_states.contains_key(name) {
                events.push(ReaderEvent::Added(name.clone()));
                // Add to state tracking with empty state and ATR
                previous_states.insert(name.clone(), (State::UNAWARE, Vec::new()));
            }
        }

        // Find removed readers
        let readers_to_remove: Vec<String> = previous_states
            .keys()
            .filter(|name| !current_names.contains(name))
            .cloned()
            .collect();

        for name in readers_to_remove {
            events.push(ReaderEvent::Removed(name.clone()));
            previous_states.remove(&name);
        }

        Ok(events)
    }

    /// Monitor for card events with a callback
    pub fn monitor_cards<H>(&self, mut handler: H) -> Result<(), PcscError>
    where
        H: CardEventHandler + Send + 'static,
    {
        let context = self.context.clone();
        let running = Arc::clone(&self.running);
        let previous_states = Arc::clone(&self.previous_states);

        // Set running flag
        {
            let mut running_guard = running.lock().unwrap();
            *running_guard = true;
        }

        thread::spawn(move || {
            let mut reader_states =
                vec![ReaderState::new(pcsc::PNP_NOTIFICATION(), State::UNAWARE)];

            // Main monitoring loop
            loop {
                // Check if we should exit
                {
                    let running_guard = running.lock().unwrap();
                    if !*running_guard {
                        break;
                    }
                }

                // Try to get updated readers list
                if let Ok(readers) = context.list_readers_owned() {
                    // Build reader states for all current readers
                    reader_states =
                        vec![ReaderState::new(pcsc::PNP_NOTIFICATION(), State::UNAWARE)];
                    for reader in readers {
                        reader_states.push(ReaderState::new(reader, State::UNAWARE));
                    }
                }

                // Update states to current
                for rs in &mut reader_states {
                    rs.sync_current_state();
                }

                // Wait for changes with timeout
                if let Ok(()) =
                    context.get_status_change(Some(Duration::from_secs(1)), &mut reader_states)
                {
                    let mut states = previous_states.lock().unwrap();

                    // Process card events
                    for rs in &reader_states {
                        let name = rs.name().to_string_lossy().into_owned();
                        let event_state = rs.event_state();

                        // Skip PnP notification
                        if name == pcsc::PNP_NOTIFICATION().to_string_lossy() {
                            continue;
                        }

                        // Card inserted
                        if event_state.contains(State::PRESENT)
                            && !event_state.contains(State::EMPTY)
                        {
                            let atr = rs.atr().to_vec();

                            // Check if this is a new insertion or a different card
                            let is_new_event = match states.get(&name) {
                                Some((prev_state, prev_atr)) => {
                                    !prev_state.contains(State::PRESENT) || *prev_atr != atr
                                }
                                None => true,
                            };

                            if is_new_event {
                                handler.handle_event(CardEvent::Inserted {
                                    reader: name.clone(),
                                    atr: atr.clone(),
                                });
                                // Update state
                                states.insert(name, (event_state, atr));
                            }
                        }
                        // Card removed
                        else if event_state.contains(State::EMPTY) {
                            let is_new_event = match states.get(&name) {
                                Some((prev_state, _)) => prev_state.contains(State::PRESENT),
                                None => false, // Don't report removal if we never saw it present
                            };

                            if is_new_event {
                                handler.handle_event(CardEvent::Removed {
                                    reader: name.clone(),
                                });
                                // Update state
                                states.insert(name, (event_state, Vec::new()));
                            }
                        }
                    }
                }

                // Small delay to prevent tight loop
                thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }

    /// Stop monitoring
    pub fn stop(&self) {
        let mut running_guard = self.running.lock().unwrap();
        *running_guard = false;
    }

    /// Monitor for card events using a channel
    pub fn monitor_cards_channel(&self, sender: CardEventSender) -> Result<(), PcscError> {
        self.monitor_cards(move |event| {
            let _ = sender.send(event);
        })
    }

    /// Monitor for reader events with a callback
    pub fn monitor_readers<H>(&self, mut handler: H) -> Result<(), PcscError>
    where
        H: ReaderEventHandler + Send + 'static,
    {
        let context = self.context.clone();
        let running = Arc::clone(&self.running);
        let previous_states = Arc::clone(&self.previous_states);

        // Set running flag
        {
            let mut running_guard = running.lock().unwrap();
            *running_guard = true;
        }

        thread::spawn(move || {
            // Main monitoring loop
            loop {
                // Check if we should exit
                {
                    let running_guard = running.lock().unwrap();
                    if !*running_guard {
                        break;
                    }
                }

                // Get current readers list
                if let Ok(current_readers) = context.list_readers_owned() {
                    let current_names: Vec<String> = current_readers
                        .iter()
                        .map(|r| r.to_string_lossy().into_owned())
                        .collect();

                    let mut states = previous_states.lock().unwrap();

                    // Find added readers
                    for name in &current_names {
                        let reader_key_exists = states.contains_key(name);
                        if !reader_key_exists {
                            handler.handle_event(ReaderEvent::Added(name.clone()));
                            // Initialize state tracking for this reader
                            states.insert(name.clone(), (State::UNAWARE, Vec::new()));
                        }
                    }

                    // Find removed readers
                    let readers_to_remove: Vec<String> = states
                        .keys()
                        .filter(|&name| !current_names.contains(name))
                        .cloned()
                        .collect();

                    for name in readers_to_remove {
                        handler.handle_event(ReaderEvent::Removed(name.clone()));
                        states.remove(&name);
                    }
                }

                // Sleep to prevent tight loop
                thread::sleep(Duration::from_secs(1));
            }
        });

        Ok(())
    }

    /// Monitor for reader events using a channel
    pub fn monitor_readers_channel(&self, sender: ReaderEventSender) -> Result<(), PcscError> {
        self.monitor_readers(move |event| {
            let _ = sender.send(event);
        })
    }

    /// Monitor for card status changes with a callback
    pub fn monitor_card_status<H>(&self, mut handler: H) -> Result<(), PcscError>
    where
        H: CardStatusEventHandler + Send + 'static,
    {
        let context = self.context.clone();
        let running = Arc::clone(&self.running);
        let previous_states = Arc::clone(&self.previous_states);

        // Set running flag
        {
            let mut running_guard = running.lock().unwrap();
            *running_guard = true;
        }

        thread::spawn(move || {
            let mut reader_states =
                vec![ReaderState::new(pcsc::PNP_NOTIFICATION(), State::UNAWARE)];
            let mut previous_card_states = std::collections::HashMap::new();

            // Main monitoring loop
            loop {
                // Check if we should exit
                {
                    let running_guard = running.lock().unwrap();
                    if !*running_guard {
                        break;
                    }
                }

                // Try to get updated readers list
                if let Ok(readers) = context.list_readers_owned() {
                    // Build reader states for all current readers
                    reader_states =
                        vec![ReaderState::new(pcsc::PNP_NOTIFICATION(), State::UNAWARE)];
                    for reader in readers {
                        reader_states.push(ReaderState::new(reader, State::UNAWARE));
                    }
                }

                // Update states to current
                for rs in &mut reader_states {
                    rs.sync_current_state();
                }

                // Wait for changes with timeout
                if let Ok(()) =
                    context.get_status_change(Some(Duration::from_secs(1)), &mut reader_states)
                {
                    // Process state changes
                    for rs in &reader_states {
                        let name = rs.name().to_string_lossy().into_owned();
                        let event_state = rs.event_state();

                        // Skip PnP notification
                        if name == pcsc::PNP_NOTIFICATION().to_string_lossy() {
                            continue;
                        }

                        // Determine card state
                        let state = if event_state.contains(State::PRESENT) {
                            CardState::Present
                        } else if event_state.contains(State::UNPOWERED) {
                            CardState::Unpowered
                        } else if event_state.contains(State::MUTE) {
                            CardState::Mute
                        } else {
                            continue; // No relevant state change
                        };

                        // Check if state changed from previous
                        let prev_state = previous_card_states.get(&name);
                        if prev_state.is_none() || prev_state.unwrap() != &state {
                            // State changed, notify
                            handler.handle_event(CardStatusEvent::StateChanged {
                                reader: name.clone(),
                                state,
                            });

                            // Update previous state
                            previous_card_states.insert(name.clone(), state);

                            // Also update our shared state tracking
                            let mut shared_states = previous_states.lock().unwrap();
                            if let Some((state_entry, _)) = shared_states.get_mut(&name) {
                                // Update state but preserve ATR
                                *state_entry = event_state;
                            }
                        }
                    }
                }

                // Small delay to prevent tight loop
                thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }

    /// Monitor for card status changes using a channel
    pub fn monitor_card_status_channel(
        &self,
        sender: CardStatusEventSender,
    ) -> Result<(), PcscError> {
        self.monitor_card_status(move |event| {
            let _ = sender.send(event);
        })
    }
}
