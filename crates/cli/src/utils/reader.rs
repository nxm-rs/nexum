use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscReader};

/// Find a reader with a specific name
pub fn find_reader_by_name(
    manager: &PcscDeviceManager,
    reader_name: &str,
) -> Result<PcscReader, Box<dyn std::error::Error>> {
    let readers = manager.list_readers()?;

    readers
        .iter()
        .find(|r| r.name() == reader_name)
        .cloned()
        .ok_or_else(|| format!("Reader '{}' not found", reader_name).into())
}

/// List all available readers
pub fn list_readers(manager: &PcscDeviceManager) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

/// Find a reader with a card inserted
pub fn find_reader_with_card(
    manager: &PcscDeviceManager,
) -> Result<PcscReader, Box<dyn std::error::Error>> {
    let readers = manager.list_readers()?;

    if readers.is_empty() {
        return Err("No readers found!".into());
    }

    // Find a reader with a card
    let reader = readers
        .iter()
        .find(|r| r.has_card())
        .ok_or("No card found in any reader!")?;

    Ok(reader.clone())
}
