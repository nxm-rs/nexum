//! Example showing how to monitor PC/SC reader and card events

use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscMonitor};
use std::collections::HashMap;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First, list all currently available readers
    let manager = PcscDeviceManager::new()?;
    let initial_readers = manager
        .list_readers()
        .map(|readers| {
            readers
                .iter()
                .map(|r| r.name().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    println!("Initially detected readers:");
    for (i, reader) in initial_readers.iter().enumerate() {
        println!("  {}. {}", i + 1, reader);
    }

    // Create a monitor
    let monitor = PcscMonitor::create()?;

    println!("\nMonitoring for reader and card events. Press Ctrl+C to exit.");
    println!("Waiting for events...");

    // Set up channels for different event types
    let (card_sender, card_receiver) = nexum_apdu_transport_pcsc::event::card_event_channel();
    let (reader_sender, reader_receiver) = nexum_apdu_transport_pcsc::event::reader_event_channel();

    // Start monitoring both card and reader events
    monitor.monitor_cards_channel(card_sender)?;
    monitor.monitor_readers_channel(reader_sender)?;

    // Track seen events to avoid duplicates
    let mut seen_cards: HashMap<String, Vec<u8>> = HashMap::new();
    let mut known_readers = initial_readers;

    // Process both kinds of events in the main thread
    loop {
        // Check for card events
        if let Ok(event) = card_receiver.try_recv() {
            match event {
                nexum_apdu_transport_pcsc::CardEvent::Inserted { reader, atr } => {
                    let is_new = match seen_cards.get(&reader) {
                        Some(prev_atr) => *prev_atr != atr,
                        None => true,
                    };

                    if is_new {
                        println!(
                            "Card inserted in reader '{}', ATR: {}",
                            reader,
                            hex::encode_upper(&atr)
                        );
                        seen_cards.insert(reader, atr);
                    }
                }
                nexum_apdu_transport_pcsc::CardEvent::Removed { reader } => {
                    if seen_cards.contains_key(&reader) {
                        println!("Card removed from reader '{}'", reader);
                        // Mark as removed but keep in map to track state
                        seen_cards.insert(reader, Vec::new());
                    }
                }
            }
        }

        // Check for reader events
        if let Ok(event) = reader_receiver.try_recv() {
            match event {
                nexum_apdu_transport_pcsc::ReaderEvent::Added(name) => {
                    if !known_readers.contains(&name) {
                        println!("Reader added: {}", name);
                        known_readers.push(name);
                    }
                }
                nexum_apdu_transport_pcsc::ReaderEvent::Removed(name) => {
                    if let Some(pos) = known_readers.iter().position(|x| *x == name) {
                        println!("Reader removed: {}", name);
                        known_readers.remove(pos);
                        seen_cards.remove(&name);
                    }
                }
            }
        }

        // Small delay to prevent 100% CPU usage
        std::thread::sleep(Duration::from_millis(100));
    }
}
