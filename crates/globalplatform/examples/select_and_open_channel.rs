//! Example to select Card Manager and open a secure channel
//!
//! This example connects to a PC/SC reader, selects the ISD, and opens a secure channel.

use nexum_apdu_globalplatform::{DefaultGlobalPlatform, commands::select::SelectOk};
use nexum_apdu_transport_pcsc::PcscDeviceManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;

    if readers.is_empty() {
        println!("No readers found!");
        return Ok(());
    }

    println!("Available readers:");
    for (i, reader) in readers.iter().enumerate() {
        let status = if reader.has_card() {
            "card present"
        } else {
            "no card"
        };
        println!("{}. {} ({})", i + 1, reader.name(), status);
    }

    // Find a reader with a card
    let reader = match readers.iter().find(|r| r.has_card()) {
        Some(reader) => reader,
        None => {
            println!("No card found in any reader!");
            return Ok(());
        }
    };

    println!("\nUsing reader: {}", reader.name());

    // Create GlobalPlatform instance
    let mut gp = DefaultGlobalPlatform::connect(reader.name())?;

    // Select the Card Manager
    println!("Selecting Card Manager...");
    let select_response = gp.select_card_manager()??;

    let SelectOk::Success { fci } = select_response;
    println!("Card Manager selected successfully.");

    // Print FCI information if available
    println!("FCI data: {}", hex::encode_upper(fci));

    // Open secure channel
    println!("\nOpening secure channel...");
    match gp.open_secure_channel() {
        Ok(_) => {
            println!("Secure channel established successfully!");
            println!("Card is ready for management operations.");
        }
        Err(e) => {
            println!("Failed to open secure channel: {:?}", e);
            println!("The card might be using non-default keys or might not support SCP02.");
        }
    }

    Ok(())
}
