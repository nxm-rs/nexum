//! Example to load a CAP file to a GlobalPlatform-compatible card
//!
//! This example connects to a PC/SC reader, selects the ISD, opens a secure channel,
//! and loads a CAP file to the card.

use std::path::PathBuf;

use apdu_core::CardExecutor;
use apdu_globalplatform::GlobalPlatform;
use apdu_transport_pcsc::{PcscConfig, PcscDeviceManager};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <cap_file_path>", args[0]);
        return Ok(());
    }

    let cap_file_path = PathBuf::from(&args[1]);
    if !cap_file_path.exists() {
        println!("CAP file not found: {:?}", cap_file_path);
        return Ok(());
    }

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

    // Connect to the reader
    let config = PcscConfig::default();
    let transport = manager.open_reader_with_config(reader.name(), config)?;
    let executor = CardExecutor::new(transport);

    // Create GlobalPlatform instance
    let mut gp = GlobalPlatform::new(executor);

    // Select the Card Manager
    println!("Selecting Card Manager...");
    let select_response = gp.select_card_manager()?;
    if !select_response.is_success() {
        println!("Failed to select Card Manager!");
        return Ok(());
    }
    println!("Card Manager selected successfully.");

    // Open secure channel
    println!("Opening secure channel...");
    match gp.open_secure_channel() {
        Ok(_) => println!("Secure channel established."),
        Err(e) => {
            println!("Failed to open secure channel: {:?}", e);
            return Ok(());
        }
    }

    // Extract package AID from filename
    let file_stem = cap_file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    println!("Loading CAP file: {:?}", cap_file_path);
    println!("Package name: {}", file_stem);

    // Prepare callback for progress reporting
    let mut callback = |current: usize, total: usize| -> apdu_globalplatform::Result<()> {
        println!(
            "Loading block {}/{} ({}%)",
            current,
            total,
            (current * 100) / total
        );
        Ok(())
    };

    // First try to delete any existing package with the same AID
    // This is just for demonstration - in a real application, you'd use proper AIDs
    let package_aid = hex::decode("A0000000030000")?;
    println!(
        "Deleting any existing package with AID: {}",
        hex::encode_upper(&package_aid)
    );

    match gp.delete_object_and_related(&package_aid) {
        Ok(_) => println!("Package deleted."),
        Err(e) => println!("Package not found or error deleting: {:?}", e),
    }

    // Install for load
    println!("Installing for load...");
    match gp.install_for_load(&package_aid, None) {
        Ok(_) => println!("Install for load successful."),
        Err(e) => {
            println!("Install for load failed: {:?}", e);
            return Ok(());
        }
    }

    // Load the CAP file
    println!("Loading CAP file...");
    match gp.load_cap_file(cap_file_path, Some(&mut callback)) {
        Ok(_) => println!("CAP file loaded successfully."),
        Err(e) => {
            println!("Failed to load CAP file: {:?}", e);
            return Ok(());
        }
    }

    // Install for install
    println!("Installing applet...");
    let applet_aid = hex::decode("A000000003000001")?;
    let instance_aid = hex::decode("A000000003000001")?;
    let params = [];

    match gp.install_for_install_and_make_selectable(
        &package_aid,
        &applet_aid,
        &instance_aid,
        &params,
    ) {
        Ok(_) => println!("Applet installed successfully."),
        Err(e) => {
            println!("Failed to install applet: {:?}", e);
            return Ok(());
        }
    }

    println!("All operations completed successfully.");
    Ok(())
}
