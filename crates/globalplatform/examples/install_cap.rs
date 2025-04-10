//! Example to install applets from a CAP file to a GlobalPlatform-compatible card
//!
//! This example connects to a PC/SC reader, selects the ISD, opens a secure channel,
//! and installs selected applets from a CAP file to the card.

use std::io::{self, Write};
use std::path::PathBuf;

use nexum_apdu_globalplatform::{DefaultGlobalPlatform, load::LoadCommandStream};
use nexum_apdu_transport_pcsc::PcscDeviceManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tracing logger with env_format and ansi
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

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

    // Create GlobalPlatform instance
    let mut gp = DefaultGlobalPlatform::connect(reader.name())?;

    // Select the Card Manager
    println!("Selecting Card Manager...");
    let _ = gp.select_card_manager()??;
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

    // Extract CAP file information
    println!("Analyzing CAP file: {:?}", cap_file_path);
    let info = LoadCommandStream::extract_info(&cap_file_path)?;

    // Display package info
    if let Some(package_aid) = &info.package_aid {
        println!("Package AID: {}", hex::encode_upper(package_aid));
    } else {
        println!("Package AID not found in CAP file!");
        return Ok(());
    }

    // Display version if available
    if let Some((major, minor)) = info.version {
        println!("Version: {}.{}", major, minor);
    }

    // Display applet AIDs
    println!("\nApplets in CAP file:");
    if info.applet_aids.is_empty() {
        println!("  No applets found!");
        return Ok(());
    } else {
        for i in 0..info.applet_aids.len() {
            let aid = &info.applet_aids[i];
            let name = if i < info.applet_names.len() {
                &info.applet_names[i]
            } else {
                "Unknown"
            };
            println!("  {}. {} - AID: {}", i + 1, name, hex::encode_upper(aid));
        }
    }

    // Interactive mode to select applets
    println!("\nSelect applets to install:");
    println!("  0. All applets");
    for i in 0..info.applet_aids.len() {
        let name = if i < info.applet_names.len() {
            &info.applet_names[i]
        } else {
            "Unknown"
        };
        println!("  {}. {}", i + 1, name);
    }

    print!("\nEnter selection (0-{}): ", info.applet_aids.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selection = input.trim().parse::<usize>().unwrap_or(0);

    let package_aid = info.package_aid.as_ref().unwrap();

    // First try to delete any existing package with the same AID
    println!("Checking for existing package...");
    match gp.delete_object_and_related(package_aid) {
        Ok(_) => println!("Existing package deleted."),
        Err(_) => println!("No existing package found or not deletable."),
    }

    // Install for load
    println!("Installing for load...");
    match gp.install_for_load(package_aid, None) {
        Ok(_) => println!("Install for load successful."),
        Err(e) => {
            println!("Install for load failed: {:?}", e);
            return Ok(());
        }
    }

    // Prepare callback for progress reporting
    let mut callback = |current: usize, total: usize| -> nexum_apdu_globalplatform::Result<()> {
        println!(
            "Loading block {}/{} ({}%)",
            current,
            total,
            (current * 100) / total
        );
        Ok(())
    };

    // Load the CAP file
    println!("Loading CAP file...");
    match gp.load_cap_file(&cap_file_path, Some(&mut callback)) {
        Ok(_) => println!("CAP file loaded successfully."),
        Err(e) => {
            println!("Failed to load CAP file: {:?}", e);
            return Ok(());
        }
    }

    // Install selected applets
    if selection == 0 {
        // Install all applets
        println!("Installing all applets...");
        for i in 0..info.applet_aids.len() {
            let applet_aid = &info.applet_aids[i];
            let name = if i < info.applet_names.len() {
                &info.applet_names[i]
            } else {
                "Unknown"
            };

            println!("Installing {}: {}", name, hex::encode_upper(applet_aid));

            match gp.install_for_install_and_make_selectable(
                package_aid,
                applet_aid,
                applet_aid, // using same AID for instance
                &[],        // empty parameters
            ) {
                Ok(_) => println!("  Installed successfully."),
                Err(e) => println!("  Installation failed: {:?}", e),
            }
        }
    } else if selection <= info.applet_aids.len() {
        // Install specific applet
        let index = selection - 1;
        let applet_aid = &info.applet_aids[index];
        let name = if index < info.applet_names.len() {
            &info.applet_names[index]
        } else {
            "Unknown"
        };

        println!("Installing {}: {}", name, hex::encode_upper(applet_aid));

        match gp.install_for_install_and_make_selectable(
            package_aid,
            applet_aid,
            applet_aid, // using same AID for instance
            &[],        // empty parameters
        ) {
            Ok(_) => println!("Applet installed successfully."),
            Err(e) => println!("Applet installation failed: {:?}", e),
        }
    } else {
        println!("Invalid selection!");
    }

    println!("All operations completed.");
    Ok(())
}
