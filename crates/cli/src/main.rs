use clap::{Parser, Subcommand};
use nexum_apdu_transport_pcsc::{PcscConfig, PcscDeviceManager};
use std::path::PathBuf;
use tracing::info;

mod commands;
mod utils;

use commands::*;
use utils::{PairingArgs, reader};

#[derive(Parser)]
#[command(version, about = "Keycard CLI for managing and using Status Keycard")]
struct Cli {
    /// Optional reader name to use (will auto-detect if not specified)
    #[arg(short, long)]
    reader: Option<String>,

    /// Trace level output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available readers
    List,

    /// Select the Keycard application and show info
    Select,

    /// Initialize a Keycard with random secrets
    Init {
        /// Optional PIN (6 digits, default is random)
        #[arg(long)]
        pin: Option<String>,

        /// Optional PUK (12 digits, default is random)
        #[arg(long)]
        puk: Option<String>,

        /// Optional pairing password (default is random)
        #[arg(long)]
        pairing_password: Option<String>,
    },

    /// Pair with a Keycard
    Pair {
        /// Pairing password
        #[arg(required = true)]
        pairing_password: String,

        /// Optional output file to save pairing info
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Open a secure channel
    OpenSecureChannel {
        /// Path to file containing pairing data
        #[arg(long, group = "pairing")]
        file: Option<PathBuf>,

        /// Pairing key in hex (must be used with --index)
        #[arg(long, requires = "index", group = "pairing")]
        key: Option<String>,

        /// Pairing index (must be used with --key)
        #[arg(long, requires = "key")]
        index: Option<u8>,
    },

    /// Verify PIN
    VerifyPin {
        /// PIN code
        #[arg(required = true)]
        pin: String,

        /// Pairing key in hex (needed if secure channel not already open)
        #[arg(long, requires = "index", group = "pairing")]
        pairing_key: Option<String>,

        /// Pairing index (needed if secure channel not already open)
        #[arg(long, requires = "pairing_key")]
        index: Option<u8>,

        /// Path to file containing pairing data
        #[arg(long, group = "pairing")]
        file: Option<PathBuf>,
    },

    /// Generate a new key pair on the card
    GenerateKey {
        /// PIN code (needed if not already verified)
        #[arg(long)]
        pin: Option<String>,

        /// Pairing key in hex (needed if secure channel not already open)
        #[arg(long, requires = "index", group = "pairing")]
        pairing_key: Option<String>,

        /// Pairing index (needed if secure channel not already open)
        #[arg(long, requires = "pairing_key")]
        index: Option<u8>,

        /// Path to file containing pairing data
        #[arg(long, group = "pairing")]
        file: Option<PathBuf>,
    },

    /// Sign data with the current key
    Sign {
        /// Data to sign, as a hex string
        #[arg(required = true)]
        data: String,

        /// Optional key derivation path
        #[arg(long)]
        path: Option<String>,

        /// PIN code (needed if not already verified)
        #[arg(long)]
        pin: Option<String>,

        /// Pairing key in hex (needed if secure channel not already open)
        #[arg(long, requires = "index", group = "pairing")]
        pairing_key: Option<String>,

        /// Pairing index (needed if secure channel not already open)
        #[arg(long, requires = "pairing_key")]
        index: Option<u8>,

        /// Path to file containing pairing data
        #[arg(long, group = "pairing")]
        file: Option<PathBuf>,
    },

    /// Export pairing info to a file
    ExportPairing {
        /// Output file path
        #[arg(short, long, required = true)]
        output: PathBuf,
    },

    /// Change PIN, PUK, or pairing secret
    ChangeCredentials {
        /// Type of credential to change: 'pin', 'puk', or 'pairing'
        #[arg(short, long, required = true)]
        credential_type: String,

        /// New value for the credential
        #[arg(short, long, required = true)]
        new_value: String,

        /// Current PIN (required for authentication)
        #[arg(long)]
        pin: Option<String>,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: PairingArgs,
    },

    /// Unblock PIN using PUK
    UnblockPin {
        /// PUK code
        #[arg(required = true)]
        puk: String,

        /// New PIN code
        #[arg(required = true)]
        new_pin: String,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: PairingArgs,
    },

    /// Set a PIN-less path for signature operations
    SetPinlessPath {
        /// Derivation path (e.g. m/44'/0'/0'/0/0)
        #[arg(required = true)]
        path: String,

        /// PIN code (needed if not already verified)
        #[arg(long)]
        pin: Option<String>,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: PairingArgs,
    },

    /// Remove the current key from the card
    RemoveKey {
        /// PIN code (needed if not already verified)
        #[arg(long)]
        pin: Option<String>,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: PairingArgs,
    },

    /// Get detailed status information
    GetStatus,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Setup logging based on verbosity
    setup_logging(cli.verbose);

    // Create a PC/SC device manager
    let manager = PcscDeviceManager::new()?;

    match &cli.command {
        Commands::List => {
            list_readers(&manager)?;
            return Ok(());
        }
        _ => {
            // For all other commands, find appropriate reader
            let reader = match &cli.reader {
                Some(reader_name) => reader::find_reader_by_name(&manager, reader_name)?,
                None => reader::find_reader_with_card(&manager)?,
            };

            info!("Using reader: {}", reader.name());

            // Execute the command using the selected reader
            let config = PcscConfig::default();
            let transport = manager.open_reader_with_config(reader.name(), config)?;

            match &cli.command {
                Commands::List => unreachable!(), // Already handled above
                Commands::Select => select_command(transport)?,
                Commands::Init {
                    pin,
                    puk,
                    pairing_password,
                } => init_command(transport, pin, puk, pairing_password)?,
                Commands::Pair {
                    pairing_password,
                    output,
                } => pair_command(transport, pairing_password, output.as_ref())?,
                Commands::OpenSecureChannel { file, key, index } => {
                    open_secure_channel_command(transport, file.as_ref(), key.as_ref(), *index)?
                }
                Commands::VerifyPin {
                    pin,
                    pairing_key,
                    index,
                    file,
                } => {
                    verify_pin_command(transport, pin, pairing_key.as_ref(), *index, file.as_ref())?
                }
                Commands::GenerateKey {
                    pin,
                    pairing_key,
                    index,
                    file,
                } => generate_key_command(
                    transport,
                    pin.as_ref(),
                    pairing_key.as_ref(),
                    *index,
                    file.as_ref(),
                )?,
                Commands::Sign {
                    data,
                    path,
                    pin,
                    pairing_key,
                    index,
                    file,
                } => {
                    sign_command(
                        transport,
                        data,
                        path.as_ref(),
                        pin.as_ref(),
                        pairing_key.as_ref(),
                        *index,
                        file.as_ref(),
                    )
                    .await?
                }
                Commands::ExportPairing { output } => export_pairing_command(transport, output)?,
                Commands::ChangeCredentials {
                    credential_type,
                    new_value,
                    pin,
                    pairing,
                } => change_credentials_command(
                    transport,
                    credential_type,
                    new_value,
                    pin.as_ref(),
                    pairing,
                )?,
                Commands::UnblockPin {
                    puk,
                    new_pin,
                    pairing,
                } => unblock_pin_command(transport, puk, new_pin, pairing)?,
                Commands::SetPinlessPath { path, pin, pairing } => {
                    set_pinless_path_command(transport, path, pin.as_ref(), pairing)?
                }
                Commands::RemoveKey { pin, pairing } => {
                    remove_key_command(transport, pin.as_ref(), pairing)?
                }
                Commands::GetStatus => get_status_command(transport)?,
            }
        }
    }

    Ok(())
}

fn setup_logging(verbose: bool) {
    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(true)
        .init();
}
