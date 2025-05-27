//! Example showing how to enumerate connected card readers

use nexum_apdu_transport_pcsc::PcscDeviceManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;

    println!("Found {} readers:", readers.len());

    for (i, reader) in readers.iter().enumerate() {
        println!("{}. Reader: {}", i + 1, reader.name());

        if reader.has_card() {
            if let Some(atr) = reader.atr() {
                println!("   Card present, ATR: {}", hex::encode_upper(atr));
            } else {
                println!("   Card present, ATR: Unknown");
            }
        } else {
            println!("   No card present");
        }
    }

    Ok(())
}
