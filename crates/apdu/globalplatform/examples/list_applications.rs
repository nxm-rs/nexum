//! Example to list all applications on a GlobalPlatform-compatible card
//!
//! This example connects to a PC/SC reader, selects the ISD, opens a secure channel,
//! and lists all applications on the card.

use nexum_apdu_globalplatform::DefaultGlobalPlatform;
use nexum_apdu_transport_pcsc::PcscDeviceManager;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .init();

    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;

    if readers.is_empty() {
        println!("No readers found!");
        return Ok(());
    }

    // Find a reader with a card
    let reader = match readers.iter().find(|r| r.has_card()) {
        Some(reader) => reader,
        None => {
            println!("No card found in any reader!");
            return Ok(());
        }
    };

    println!("Using reader: {}", reader.name());

    // Create GlobalPlatform instance
    let mut gp = DefaultGlobalPlatform::connect(reader.name())?;

    // Select the Card Manager
    println!("Selecting Card Manager...");
    let _ = gp.select_card_manager()?;
    println!("Card Manager selected successfully.");

    // Open secure channel
    println!("Opening secure channel...");
    match gp.open_secure_channel() {
        Ok(_) => println!("Secure channel established."),
        Err(e) => {
            println!("Failed to open secure channel: {e:?}");
            return Ok(());
        }
    }

    // Get applications status
    println!("Getting applications status...");
    let response = gp.get_applications_status()?;

    let data = response.tlv_data();
    println!("Applications:");
    print_applications(data.as_slice());

    // Get load files status
    println!("\nGetting load files status...");
    let response = gp.get_load_files_status()?;

    let data = response.tlv_data();
    println!("Load files:");
    print_load_files(data.as_slice());

    Ok(())
}

/// Parse and print application information from TLV data
fn print_applications(tlv_data: &[u8]) {
    // Very simple TLV parser for demonstration
    let mut index = 0;
    while index < tlv_data.len() {
        // Look for application entries (E3 tag)
        if tlv_data[index] == 0xE3 {
            let len = tlv_data[index + 1] as usize;
            let end = index + 2 + len;

            if end <= tlv_data.len() {
                let entry = &tlv_data[index + 2..end];

                // Find AID (4F tag)
                if let Some(aid) = find_tlv_value(entry, 0x4F) {
                    println!("  AID: {}", hex::encode_upper(aid));
                }

                // Find life cycle (C5 tag)
                if let Some(lifecycle) = find_tlv_value(entry, 0xC5)
                    && !lifecycle.is_empty() {
                        println!("  Life Cycle: {:#04X}", lifecycle[0]);
                    }

                // Find privileges (C6 tag)
                if let Some(privileges) = find_tlv_value(entry, 0xC6)
                    && !privileges.is_empty() {
                        println!("  Privileges: {}", hex::encode_upper(privileges));
                    }

                println!();
            }
        }

        // Move to next TLV
        index += 1;
    }
}

/// Parse and print load file information from TLV data
fn print_load_files(tlv_data: &[u8]) {
    // Very simple TLV parser for demonstration
    let mut index = 0;
    while index < tlv_data.len() {
        // Look for load file entries (E2 tag)
        if tlv_data[index] == 0xE2 {
            let len = tlv_data[index + 1] as usize;
            let end = index + 2 + len;

            if end <= tlv_data.len() {
                let entry = &tlv_data[index + 2..end];

                // Find AID (4F tag)
                if let Some(aid) = find_tlv_value(entry, 0x4F) {
                    println!("  AID: {}", hex::encode_upper(aid));
                }

                // Find life cycle (C5 tag)
                if let Some(lifecycle) = find_tlv_value(entry, 0xC5)
                    && !lifecycle.is_empty() {
                        println!("  Life Cycle: {:#04X}", lifecycle[0]);
                    }

                println!();
            }
        }

        // Move to next TLV
        index += 1;
    }
}

/// Find a TLV value by tag
fn find_tlv_value(data: &[u8], tag: u8) -> Option<&[u8]> {
    let mut index = 0;
    while index + 1 < data.len() {
        let current_tag = data[index];
        let len = data[index + 1] as usize;

        if current_tag == tag && index + 2 + len <= data.len() {
            return Some(&data[index + 2..index + 2 + len]);
        }

        index += 2 + len;
    }

    None
}
