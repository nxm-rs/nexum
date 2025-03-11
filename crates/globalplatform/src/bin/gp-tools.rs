//! Command-line tool for GlobalPlatform card management
//!
//! This binary provides a command-line interface for common GlobalPlatform
//! operations like listing applications, installing or deleting packages, etc.

use apdu_core::{CardExecutor, ResponseAwareExecutor, SecureChannelExecutor, StatusWord};
use apdu_globalplatform::{DefaultKeys, GlobalPlatform, Keys, operations};
use apdu_transport_pcsc::{PcscConfig, PcscDeviceManager};
use clap::{Parser, Subcommand};
use hex::FromHex;
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional specific reader to use
    #[arg(short, long)]
    reader: Option<String>,

    /// Use default keys
    #[arg(short, long)]
    default_keys: bool,

    /// Specify custom keys (hex)
    #[arg(short, long)]
    keys: Option<String>,

    /// PC/SC protocol (0=T=0, 1=T=1)
    #[arg(short, long, default_value_t = 1)]
    protocol: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List detected readers
    ListReaders,

    /// List applications on the card
    ListApps,

    /// List packages on the card
    ListPackages,

    /// Delete a package and related applications
    Delete {
        /// AID to delete (hex)
        aid: String,
    },

    /// Install a CAP file
    Install {
        /// Path to the CAP file
        #[arg(short, long)]
        cap: PathBuf,

        /// Make applets selectable after installation
        #[arg(short, long)]
        make_selectable: bool,

        /// Custom install parameters (hex)
        #[arg(short, long)]
        params: Option<String>,
    },

    /// Get card identification data
    Info,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Create PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    // Handle the ListReaders command first
    if let Commands::ListReaders = cli.command {
        let readers = manager.list_readers()?;
        if readers.is_empty() {
            println!("No readers found.");
        } else {
            println!("Available readers:");
            for (i, reader) in readers.iter().enumerate() {
                let status = if reader.has_card() {
                    "card present"
                } else {
                    "no card"
                };
                println!("{}. {} ({})", i + 1, reader.name(), status);
            }
        }
        return Ok(());
    }

    // For all other commands, we need to select a reader with a card
    let readers = manager.list_readers()?;

    // Use specified reader or find first one with a card
    let reader = if let Some(reader_name) = &cli.reader {
        // Find the specified reader
        readers
            .iter()
            .find(|r| r.name() == reader_name)
            .ok_or("Specified reader not found")?
    } else {
        // Find first reader with a card
        readers
            .iter()
            .find(|r| r.has_card())
            .ok_or("No reader with a card found")?
    };

    println!("Using reader: {}", reader.name());

    // Set up PC/SC configuration
    let mut config = PcscConfig::default();
    // config.protocol = match cli.protocol {
    //     0 => apdu_transport_pcsc::Protocol::T0,
    //     _ => apdu_transport_pcsc::Protocol::T1,
    // };
    // config.timeout = Some(Duration::from_secs(20));

    // Connect to the card
    let transport = manager.open_reader_with_config(reader.name(), config)?;
    let executor = CardExecutor::new(transport);

    // Create GlobalPlatform instance
    let mut gp = GlobalPlatform::new(executor);

    // Select the card manager
    println!("Selecting Card Manager...");
    let select_response = gp.select_card_manager()?;
    if !select_response.is_success() {
        eprintln!("Failed to select Card Manager!");
        return Ok(());
    }

    // Open secure channel with appropriate keys
    println!("Opening secure channel...");
    let keys = if cli.default_keys {
        DefaultKeys::new()
    } else if let Some(key_str) = cli.keys {
        let key_bytes = Vec::from_hex(&key_str.replace(' ', ""))?;
        if key_bytes.len() != 16 {
            return Err("Key must be 16 bytes (32 hex characters)".into());
        }
        let mut key = [0u8; 16];
        key.copy_from_slice(&key_bytes);
        Keys::from_single_key(key)
    } else {
        // Default to test keys
        DefaultKeys::new()
    };

    gp.open_secure_channel_with_keys(
        &keys,
        apdu_core::processor::secure::SecurityLevel::MACProtection,
    )?;
    println!("Secure channel opened successfully.");

    // Process the command
    match cli.command {
        Commands::ListReaders => unreachable!(), // Already handled

        Commands::ListApps => {
            println!("\nApplications on card:");
            println!("=====================");

            let apps = operations::list_applications(&mut gp)?;
            if apps.is_empty() {
                println!("No applications found.");
            } else {
                for (i, app) in apps.iter().enumerate() {
                    println!("{}. AID: {}", i + 1, hex::encode_upper(&app.aid));
                    println!("   Lifecycle: {:#04X}", app.lifecycle);
                    if !app.privileges.is_empty() {
                        println!("   Privileges: {}", hex::encode_upper(&app.privileges));
                    }
                    println!();
                }
            }
        }

        Commands::ListPackages => {
            println!("\nPackages on card:");
            println!("================");

            let packages = operations::list_packages(&mut gp)?;
            if packages.is_empty() {
                println!("No packages found.");
            } else {
                for (i, pkg) in packages.iter().enumerate() {
                    println!("{}. AID: {}", i + 1, hex::encode_upper(&pkg.aid));
                    println!("   Lifecycle: {:#04X}", pkg.lifecycle);

                    if !pkg.modules.is_empty() {
                        println!("   Modules:");
                        for (j, module) in pkg.modules.iter().enumerate() {
                            println!("     {}. {}", j + 1, hex::encode_upper(module));
                        }
                    }
                    println!();
                }
            }
        }

        Commands::Delete { aid } => {
            let aid_bytes = Vec::from_hex(&aid.replace(' ', ""))?;
            println!(
                "\nDeleting package with AID: {}",
                hex::encode_upper(&aid_bytes)
            );

            match operations::delete_package(&mut gp, &aid_bytes) {
                Ok(_) => println!("Package deleted successfully."),
                Err(e) => println!("Failed to delete package: {}", e),
            }
        }

        Commands::Install {
            cap,
            make_selectable,
            params,
        } => {
            println!("\nInstalling CAP file: {}", cap.display());

            // Parse install parameters if provided
            let install_params = if let Some(param_str) = params {
                Vec::from_hex(&param_str.replace(' ', ""))?
            } else {
                Vec::new()
            };

            match operations::install_cap_file(&mut gp, &cap, make_selectable, &install_params) {
                Ok(_) => println!("CAP file installed successfully."),
                Err(e) => println!("Failed to install CAP file: {}", e),
            }
        }

        Commands::Info => {
            println!("\nCard Information:");
            println!("================");

            // Get card data
            match gp.get_card_data() {
                Ok(data) => {
                    println!("Card Data: {}", hex::encode_upper(&data));

                    // Try to parse CPLC data
                    if data.len() >= 3 && data[0] == 0x66 {
                        println!("\nCard Production Life Cycle Data:");
                        let cplc_len = data[1] as usize;
                        if data.len() >= 2 + cplc_len {
                            let cplc = &data[2..2 + cplc_len];

                            // Try to extract IC manufacturer
                            if cplc.len() >= 2 {
                                println!("IC Manufacturer: {}", hex::encode_upper(&cplc[0..2]));
                            }

                            // Extract card serial number if available
                            if cplc.len() >= 10 {
                                println!("Card Serial Number: {}", hex::encode_upper(&cplc[4..10]));
                            }
                        }
                    }
                }
                Err(e) => println!("Failed to get card information: {}", e),
            }
        }
    }

    // Close secure channel
    println!("\nClosing secure channel...");
    let _ = gp.close_secure_channel();

    Ok(())
}
