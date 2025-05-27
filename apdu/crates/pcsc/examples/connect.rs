//! Example showing basic connection to a smart card and sending commands

use nexum_apdu_core::prelude::*;
use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscTransport};
use std::any::Any;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;
    if readers.is_empty() {
        println!("No readers found!");
        return Ok(());
    }

    println!("Found {} readers:", readers.len());
    for (i, reader) in readers.iter().enumerate() {
        println!("  {}. {}", i + 1, reader.name());
    }

    // Use the first reader that has a card
    let reader = match readers.iter().find(|r| r.has_card()) {
        Some(reader) => reader,
        None => {
            println!("No reader with a card inserted found!");
            return Ok(());
        }
    };

    println!("\nUsing reader: {}", reader.name());

    // Connect to the reader
    let transport = manager.open_reader(reader.name())?;
    let mut executor: CardExecutor<PcscTransport> = CardExecutor::new(transport);

    // Get ATR if available
    let transport = executor.transport();
    if let Some(pcsc_transport) = (transport as &dyn Any).downcast_ref::<PcscTransport>() {
        if let Ok(atr) = pcsc_transport.atr() {
            println!("Card ATR: {}", hex::encode_upper(&atr));
        }
    }

    // Define some common APDUs to try
    let commands = [
        // SELECT PSE (Payment System Environment)
        ("SELECT PSE", "00A404000E315041592E5359532E4444463031"),
        // SELECT PPSE (Proximity Payment System Environment)
        ("SELECT PPSE", "00A404000E325041592E5359532E4444463031"),
        // GET PROCESSING OPTIONS (VISA simplified)
        ("GPO", "80A8000002830000"),
        // Get Data - Application Interchange Profile
        ("GET DATA - AIP", "80CA9F1700"),
    ];

    // Try each command
    for (name, hex) in &commands {
        let cmd_bytes = hex::decode(hex)?;
        println!("\nSending {}: {}", name, hex);

        match executor.transmit_raw(&cmd_bytes) {
            Ok(response_bytes) => match Response::from_bytes(&response_bytes) {
                Ok(response) => {
                    println!("Response:");
                    println!("  Status: {}", response.status());
                    if let Some(payload) = response.payload() {
                        println!("  Data: {}", hex::encode_upper(payload));
                    }
                }
                Err(e) => println!("Error parsing response: {:?}", e),
            },
            Err(e) => println!("Command failed: {:?}", e),
        }

        // Add a small delay between commands to let the card stabilize
        sleep(Duration::from_millis(50));
    }

    // Create a custom command
    let aid = hex::decode("A000000003000000")?; // VISA AID
    let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid);

    println!("\nSelecting VISA AID:");
    match executor.transmit_raw(&select_cmd.to_bytes()) {
        Ok(response) => {
            println!(
                "Response: {} bytes, status: {}",
                response.len() - 2, // Subtract 2 for status bytes
                hex::encode_upper(&response[response.len() - 2..])
            );

            if response.len() > 2 {
                println!(
                    "Data: {}",
                    hex::encode_upper(&response[..response.len() - 2])
                );
            }
        }
        Err(e) => println!("Command failed: {:?}", e),
    }

    // Reset the card before exiting to put it in a clean state
    if let Err(e) = executor.reset() {
        println!("Warning: Failed to reset card: {:?}", e);
    }

    println!("\nConnection test completed.");
    Ok(())
}
