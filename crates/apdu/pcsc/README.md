# APDU PC/SC Transport

A Rust implementation of the PC/SC (Personal Computer/Smart Card) transport layer for APDU operations.

## Overview

This crate provides a PC/SC-based implementation of the `CardTransport` trait from `nexum-apdu-core`, enabling communication with smart cards through PC/SC readers. It offers a complete interface for connecting to smart card readers, managing card sessions, and monitoring reader/card events.

## Features

- Connect to PC/SC compatible smart card readers
- Transmit APDUs to smart cards
- Monitor reader and card events (insertion/removal)
- Support for channel-based event handling
- Transaction management
- Comprehensive error handling

## Usage Example

```rust
use nexum_apdu_core::prelude::*;
use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscConfig};

fn main() -> Result<(), Error> {
    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;
    if readers.is_empty() {
        println!("No readers found");
        return Ok(());
    }

    // Connect to the first reader with a card
    let reader = readers.iter().find(|r| r.has_card()).expect("No card present");
    println!("Using reader: {}", reader.name());

    // Display ATR if available
    if let Some(atr) = reader.atr() {
        println!("Card ATR: {}", hex::encode_upper(atr));
    }

    // Connect to the reader
    let transport = manager.open_reader(reader.name())?;
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Send a SELECT command
    let aid = hex::decode("A000000003000000").unwrap(); // Example AID
    let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid);

    match executor.transmit(&select_cmd) {
        Ok(response) => {
            println!("Response: {:?}", response);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    Ok(())
}
```

## Event Monitoring Example

```rust
use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscMonitor};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a monitor
    let monitor = PcscMonitor::create()?;

    // Set up channel for card events
    let (sender, receiver) = nexum_apdu_transport_pcsc::event::card_event_channel();

    // Start monitoring
    monitor.monitor_cards_channel(sender)?;

    println!("Monitoring for card events. Press Ctrl+C to exit.");

    // Process events in main thread
    loop {
        if let Ok(event) = receiver.recv_timeout(Duration::from_millis(100)) {
            match event {
                nexum_apdu_transport_pcsc::CardEvent::Inserted { reader, atr } => {
                    println!("Card inserted in {}, ATR: {}", reader, hex::encode_upper(&atr));
                }
                nexum_apdu_transport_pcsc::CardEvent::Removed { reader } => {
                    println!("Card removed from {}", reader);
                }
            }
        }
    }
}
```

## Configuration Options

The crate provides flexible configuration options through the `PcscConfig` struct:

```rust
// Create a custom configuration
let config = PcscConfig::default()
    .with_share_mode(ShareMode::Exclusive)
    .with_protocols(pcsc::Protocols::T1)
    .with_auto_reconnect(true)
    .with_transaction_mode(TransactionMode::PerCommand);

// Open reader with custom config
let transport = manager.open_reader_with_config(reader.name(), config)?;
```

## Using with the Core Prelude

This transport layer integrates smoothly with the `nexum-apdu-core` prelude:

```rust
use nexum_apdu_core::prelude::*;
use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscConfig};

fn main() -> Result<(), Error> {
    // Create transport
    let manager = PcscDeviceManager::new()?;
    let readers = manager.list_readers()?;
    let reader = readers.iter().find(|r| r.has_card()).expect("No card present");
    let transport = manager.open_reader(reader.name())?;

    // Create executor and use all core functionality
    let mut executor = CardExecutor::new_with_defaults(transport);

    // Rest of your code...
    Ok(())
}
```

## Included Examples

The crate comes with several examples to help you get started:

- `list_readers.rs` - Lists all available PC/SC readers
- `connect.rs` - Basic connection and APDU transmission
- `select_aid.rs` - Example of selecting applications by AID
- `monitor_events.rs` - Monitors and displays reader/card events
- `apdu_shell.rs` - Interactive shell for sending APDUs to a card

## License

Licensed under the [AGPL License](../../LICENSE) or http://www.gnu.org/licenses/agpl-3.0.html.

## Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in these crates by you shall be licensed as above, without any additional terms or conditions.
