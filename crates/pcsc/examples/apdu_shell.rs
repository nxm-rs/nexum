//! Interactive APDU shell for sending commands to a smart card

use nexum_apdu_core::prelude::Executor;
use nexum_apdu_core::{ApduCommand, ApduResponse, CardExecutor, Command, Response};
use nexum_apdu_transport_pcsc::{PcscConfig, PcscDeviceManager, PcscTransport};
use std::any::Any;
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;

    if readers.is_empty() {
        println!("No readers found!");
        return Ok(());
    }

    // Display readers for selection
    println!("Available readers:");
    for (i, reader) in readers.iter().enumerate() {
        let card_status = if reader.has_card() {
            "card present"
        } else {
            "no card"
        };
        println!("{}. {} ({})", i + 1, reader.name(), card_status);
    }

    // Select a reader
    print!("Select a reader (1-{}): ", readers.len());
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let reader_index = match lines.next() {
        Some(Ok(input)) => match input.trim().parse::<usize>() {
            Ok(index) if index > 0 && index <= readers.len() => index - 1,
            _ => {
                println!("Invalid selection, using first reader");
                0
            }
        },
        _ => {
            println!("Invalid input, using first reader");
            0
        }
    };

    let reader = &readers[reader_index];
    println!("Using reader: {}", reader.name());

    if !reader.has_card() {
        println!("No card present in the selected reader!");
        return Ok(());
    }

    // Connect to the reader
    let config = PcscConfig::default();
    let transport = manager.open_reader_with_config(reader.name(), config)?;
    let mut executor: CardExecutor<PcscTransport> = CardExecutor::new(transport);

    println!("\nAPDU Shell - Enter commands in hex format or 'help' for assistance");
    println!("Examples:");
    println!("  00A404000AA000000003000000");
    println!("  00 A4 04 00 0A A0 00 00 00 03 00 00 00");

    loop {
        print!("> ");
        io::stdout().flush()?;

        let line = match lines.next() {
            Some(Ok(input)) => input,
            _ => break,
        };

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        // Process commands
        match input.to_lowercase().as_str() {
            "exit" | "quit" | "q" => break,

            "help" | "?" => {
                println!("Commands:");
                println!("  <hex>     - Send APDU command (e.g., '00A4040008A000000003000000')");
                println!("  select    - Execute SELECT command with provided AID");
                println!("  reset     - Reset the card connection");
                println!("  atr       - Display the card's ATR");
                println!("  help      - Show this help");
                println!("  exit      - Exit the shell");
            }

            "atr" => {
                // Get a reference to the transport
                let transport = executor.transport();

                // Try to downcast to our specific transport type
                if let Some(pcsc_transport) =
                    (transport as &dyn Any).downcast_ref::<PcscTransport>()
                {
                    match pcsc_transport.atr() {
                        Ok(atr) => println!("ATR: {}", hex::encode_upper(&atr)),
                        Err(e) => println!("Error getting ATR: {:?}", e),
                    }
                } else {
                    println!("Could not access transport");
                }
            }

            "reset" => {
                // Reset both the executor and transport
                match executor.reset() {
                    Ok(_) => println!("Card reset successfully"),
                    Err(e) => println!("Error resetting card: {:?}", e),
                }
            }

            cmd if cmd.starts_with("select") => {
                // Extract AID from command
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() != 2 {
                    println!("Usage: select <aid-in-hex>");
                    continue;
                }

                // Parse AID
                let aid = match hex::decode(parts[1].replace(' ', "")) {
                    Ok(aid) => aid,
                    Err(_) => {
                        println!("Invalid AID format");
                        continue;
                    }
                };

                // Create SELECT command
                let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid.clone());

                println!("Selecting AID: {}", hex::encode_upper(&aid));

                // Execute command
                match executor.transmit_raw(&select_cmd.to_bytes()) {
                    Ok(response_bytes) => {
                        // Clone to avoid borrow issues with Response::from_bytes
                        let response_data = response_bytes.to_vec();
                        match Response::from_bytes(&response_data) {
                            Ok(response) => {
                                println!("Response:");
                                println!("  Status: {}", response.status());
                                println!("  Data: {}", hex::encode_upper(response.payload()));
                            }
                            Err(e) => println!("Error parsing response: {:?}", e),
                        }
                    }
                    Err(e) => println!("Command failed: {:?}", e),
                }
            }

            // Treat as raw APDU
            _ => {
                // Remove any spaces
                let clean_input = input.replace(' ', "");

                // Parse hex
                match hex::decode(&clean_input) {
                    Ok(command_bytes) => {
                        if command_bytes.len() < 4 {
                            println!("APDU command too short");
                            continue;
                        }

                        // Execute raw APDU
                        match executor.transmit_raw(&command_bytes) {
                            Ok(response_bytes) => {
                                // Clone to avoid borrow issues with Response::from_bytes
                                let response_data = response_bytes.to_vec();
                                match Response::from_bytes(&response_data) {
                                    Ok(response) => {
                                        println!("Response:");
                                        println!("  Status: {}", response.status());
                                        if !response.payload().is_empty() {
                                            println!(
                                                "  Data: {}",
                                                hex::encode_upper(response.payload())
                                            );
                                        }
                                    }
                                    Err(e) => println!("Error parsing response: {:?}", e),
                                }
                            }
                            Err(e) => println!("Command failed: {:?}", e),
                        }
                    }
                    Err(_) => println!("Invalid hex input"),
                }
            }
        }
    }

    println!("Goodbye!");
    Ok(())
}
