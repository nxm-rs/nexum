//! Card reader handling utilities

use nexum_apdu_transport_pcsc::PcscDeviceManager;
use std::error::Error;
use tracing::debug;

/// Find a reader by name
pub fn find_reader_by_name(
    manager: &PcscDeviceManager,
    reader_name: &str,
) -> Result<String, Box<dyn Error>> {
    let readers = manager.list_readers()?;
    debug!("Found {} readers", readers.len());

    for reader in readers {
        debug!("Reader: {}", reader.name());
        if reader.name().contains(reader_name) {
            return Ok(reader.name().to_string());
        }
    }

    Err(format!("No reader matching '{}' found", reader_name).into())
}

/// Find first reader with a card present
pub fn find_reader_with_card(manager: &PcscDeviceManager) -> Result<String, Box<dyn Error>> {
    let readers = manager.list_readers()?;
    debug!("Found {} readers", readers.len());

    for reader in readers {
        debug!("Reader: {}", reader.name());
        if reader.has_card() {
            return Ok(reader.name().to_string());
        }
    }

    Err("No reader with a card present found".into())
}

/// List all available readers
pub fn list_readers(manager: &PcscDeviceManager) -> Result<(), Box<dyn Error>> {
    let readers = manager.list_readers()?;

    if readers.is_empty() {
        println!("No readers found");
        return Ok(());
    }

    println!("Found {} readers:", readers.len());

    for (i, reader) in readers.iter().enumerate() {
        let card_status = if reader.has_card() {
            "Card present"
        } else {
            "No card"
        };

        println!("{}. {} ({})", i + 1, reader.name(), card_status);
    }

    Ok(())
}
