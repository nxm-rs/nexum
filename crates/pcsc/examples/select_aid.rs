//! Example showing how to select an application by AID

use nexum_apdu_core::prelude::Executor;
use nexum_apdu_core::transport::error::TransportError;
use nexum_apdu_core::{ApduCommand, ApduResponse, CardExecutor, Command, Error};
use nexum_apdu_transport_pcsc::PcscDeviceManager;
use std::thread::sleep;
use std::time::Duration;

// Define common AIDs (Application Identifiers)
struct AidRegistry;

impl AidRegistry {
    // Payment AIDs
    const VISA: &'static str = "A0000000031010";
    const MASTERCARD: &'static str = "A0000000041010";
    const AMEX: &'static str = "A000000025010801";
    const DISCOVER: &'static str = "A0000001523010";
    const JCB: &'static str = "A0000000651010";

    // Identity/Government AIDs
    const PIV: &'static str = "A0000003081000";

    // Transport AIDs
    const ORCA: &'static str = "A0000004040125";

    // Telecom AIDs
    const GSM: &'static str = "A0000000871002";

    // Other AIDs
    const OPENSC_PKCS15: &'static str = "A000000167455349474E";
    const OPENPGP: &'static str = "D27600012401";
}

/// Select an application by AID
fn select_aid(
    executor: &mut CardExecutor<impl nexum_apdu_core::CardTransport<Error = TransportError>>,
    aid_hex: &str,
) -> Result<String, Error> {
    let aid = hex::decode(aid_hex).map_err(|_| Error::Parse("Invalid AID hex"))?;

    // Create SELECT command with AID
    let select_cmd = Command::new_with_data(0x00, 0xA4, 0x04, 0x00, aid.clone());

    println!("Selecting AID: {}", aid_hex);

    // Send command and receive response
    let response = executor.transmit(&select_cmd.to_bytes())?;

    // Parse response as a Response object
    let resp = nexum_apdu_core::Response::from_bytes(&response)?;

    if resp.is_success() {
        Ok(format!(
            "Selected successfully, {} data bytes returned",
            resp.payload().len()
        ))
    } else {
        Ok(format!("Selection failed: {}", resp.status()))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // List available readers
    let readers = manager.list_readers()?;

    if readers.is_empty() {
        println!("No readers found!");
        return Ok(());
    }

    // Use the first reader with a card
    let reader = match readers.iter().find(|r| r.has_card()) {
        Some(reader) => reader,
        None => {
            println!("No card present in any reader!");
            return Ok(());
        }
    };

    println!("Using reader: {}", reader.name());

    // Connect to the reader
    let transport = manager.open_reader(reader.name())?;
    let mut executor = CardExecutor::new(transport);

    // Try to select various common AIDs
    println!("\nTrying to select common applications...");

    let aids = [
        ("Visa", AidRegistry::VISA),
        ("Mastercard", AidRegistry::MASTERCARD),
        ("American Express", AidRegistry::AMEX),
        ("Discover", AidRegistry::DISCOVER),
        ("JCB", AidRegistry::JCB),
        ("PIV Card", AidRegistry::PIV),
        ("GSM SIM", AidRegistry::GSM),
        ("ORCA Transit", AidRegistry::ORCA),
        ("OPENSC PKCS15", AidRegistry::OPENSC_PKCS15),
        ("OpenPGP", AidRegistry::OPENPGP),
    ];

    for (name, aid) in &aids {
        print!("{:<20}: ", name);
        match select_aid(&mut executor, aid) {
            Ok(result) => println!("{}", result),
            Err(e) => println!("Error: {:?}", e),
        }

        // Add a small delay between operations to allow the card to stabilize
        sleep(Duration::from_millis(50));
    }

    println!("\nAID selection test completed.");

    // Reset the card before exiting to put it in a clean state
    if let Err(e) = executor.reset() {
        println!("Warning: Failed to reset card: {:?}", e);
    }

    Ok(())
}
