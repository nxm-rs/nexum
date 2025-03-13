//! Command-line tool for GlobalPlatform card management
//!
//! This binary provides a command-line interface for common GlobalPlatform
//! operations like listing applications, installing or deleting packages, etc.

use apdu_core::CardExecutor;
use apdu_globalplatform::crypto::Scp02;
use apdu_globalplatform::{DefaultKeys, GlobalPlatform, Keys, load::LoadCommandStream, operations};
use apdu_transport_pcsc::{PcscConfig, PcscDeviceManager};
use cipher::Key;
use clap::{Parser, Subcommand, ValueEnum};
use hex::FromHex;
use std::io::{self, Write};
use std::path::PathBuf;

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
        #[arg(conflicts_with = "all")]
        aid: Option<String>,

        /// Delete all packages and applications (requires confirmation)
        #[arg(long, conflicts_with = "aid")]
        all: bool,

        /// Skip confirmation for delete operations
        #[arg(long)]
        force: bool,
    },

    /// Install a CAP file
    Install {
        /// Path to the CAP file
        #[arg(short, long)]
        cap: PathBuf,

        /// Custom install parameters (hex)
        #[arg(short, long)]
        params: Option<String>,

        /// Applet selection mode
        #[arg(short, long, value_enum, default_value = "interactive")]
        mode: InstallMode,

        /// Specific applet index to install (only used with 'specific' mode)
        #[arg(short, long)]
        index: Option<usize>,

        /// Delete existing package before installation
        #[arg(short, long, default_value_t = true)]
        delete_existing: bool,
    },

    /// Get card identification data
    Info,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum InstallMode {
    /// Interactive selection of applets
    Interactive,
    /// Install all applets in the CAP file
    All,
    /// Install a specific applet by index
    Specific,
}

fn get_user_confirmation(message: &str) -> bool {
    print!("{} (y/N): ", message);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the tracing logger with env_format and ansi
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(true)
        .init();

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
    let config = PcscConfig::default();

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
    println!("Card Manager selected successfully.");

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
        let key = Key::<Scp02>::from_slice(&key);
        Keys::from_single_key(*key)
    } else {
        // Default to test keys
        DefaultKeys::new()
    };

    match gp.open_secure_channel_with_keys(&keys) {
        Ok(_) => println!("Secure channel established."),
        Err(e) => {
            eprintln!("Failed to open secure channel: {:?}", e);
            return Ok(());
        }
    }

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

        Commands::Delete { aid, all, force } => {
            if all {
                println!(
                    "\n⚠️  WARNING: You are about to delete ALL packages and applications on the card!"
                );
                println!(
                    "This operation cannot be undone and may render the card unusable if system applications are removed."
                );

                // Get confirmation unless --force is specified
                let confirmed =
                    force || get_user_confirmation("Are you sure you want to continue?");

                if confirmed {
                    let mut success_count = 0;
                    let mut failed_count = 0;

                    // First delete standalone applications
                    println!("\nRetrieving list of all applications...");
                    let applications = operations::list_applications(&mut gp)?;

                    if !applications.is_empty() {
                        println!("Deleting {} applications:", applications.len());

                        for app in applications {
                            println!("Deleting application: {}", hex::encode_upper(&app.aid));
                            match gp.delete_object(&app.aid) {
                                Ok(_) => {
                                    println!("  ✅ Application deleted successfully.");
                                    success_count += 1;
                                }
                                Err(e) => {
                                    println!("  ❌ Failed to delete application: {}", e);
                                    failed_count += 1;
                                }
                            }
                        }
                    } else {
                        println!("No standalone applications found on the card.");
                    }

                    // Then delete packages
                    println!("\nRetrieving list of all packages...");
                    let packages = operations::list_packages(&mut gp)?;

                    if !packages.is_empty() {
                        println!("Deleting {} packages:", packages.len());

                        for pkg in packages {
                            println!("Deleting package: {}", hex::encode_upper(&pkg.aid));
                            match operations::delete_package(&mut gp, &pkg.aid) {
                                Ok(_) => {
                                    println!("  ✅ Package deleted successfully.");
                                    success_count += 1;
                                }
                                Err(e) => {
                                    println!("  ❌ Failed to delete package: {}", e);
                                    failed_count += 1;
                                }
                            }
                        }
                    } else {
                        println!("No packages found on the card.");
                    }

                    println!("\nDeletion summary:");
                    println!("  Successfully deleted: {}", success_count);
                    println!("  Failed to delete: {}", failed_count);
                } else {
                    println!("Operation cancelled.");
                }
            } else if let Some(aid_str) = aid {
                let aid_bytes = Vec::from_hex(&aid_str.replace(' ', ""))?;
                println!(
                    "\nDeleting package with AID: {}",
                    hex::encode_upper(&aid_bytes)
                );

                match operations::delete_package(&mut gp, &aid_bytes) {
                    Ok(_) => println!("Package deleted successfully."),
                    Err(e) => println!("Failed to delete package: {}", e),
                }
            } else {
                return Err("Either --all flag or a specific AID must be provided".into());
            }
        }

        Commands::Install {
            cap,
            params,
            mode,
            index,
            delete_existing,
        } => {
            // Verify the CAP file exists
            if !cap.exists() {
                println!("CAP file not found: {:?}", cap);
                return Ok(());
            }

            println!("\nInstalling CAP file: {}", cap.display());

            // Extract CAP file information
            println!("Analyzing CAP file...");
            let info = LoadCommandStream::extract_info(&cap)?;

            // Display package info
            let package_aid = if let Some(aid) = &info.package_aid {
                println!("Package AID: {}", hex::encode_upper(aid));
                aid
            } else {
                println!("Package AID not found in CAP file!");
                return Ok(());
            };

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

            // Determine which applets to install
            let selection = match mode {
                InstallMode::Interactive => {
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
                    input.trim().parse::<usize>().unwrap_or(0)
                }
                InstallMode::All => 0, // 0 means all applets
                InstallMode::Specific => {
                    if let Some(idx) = index {
                        if idx <= info.applet_aids.len() {
                            idx
                        } else {
                            println!("Invalid applet index. Using first applet.");
                            1
                        }
                    } else {
                        println!(
                            "No applet index specified for 'specific' mode. Using first applet."
                        );
                        1
                    }
                }
            };

            // Parse install parameters if provided
            let install_params = if let Some(param_str) = params {
                Vec::from_hex(&param_str.replace(' ', ""))?
            } else {
                Vec::new()
            };

            // First try to delete any existing package with the same AID if requested
            if delete_existing {
                println!("Checking for existing package...");
                match gp.delete_object_and_related(package_aid) {
                    Ok(_) => println!("Existing package deleted."),
                    Err(_) => println!("No existing package found or not deletable."),
                }
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
            let mut callback = |current: usize, total: usize| -> apdu_globalplatform::Result<()> {
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
            match gp.load_cap_file(&cap, Some(&mut callback)) {
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
                        &install_params,
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
                    &install_params,
                ) {
                    Ok(_) => println!("Applet installed successfully."),
                    Err(e) => println!("Applet installation failed: {:?}", e),
                }
            } else {
                println!("Invalid selection!");
            }

            println!("Installation process completed.");
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

    println!("All operations completed.");
    Ok(())
}
