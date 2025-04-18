use clap::Subcommand;
use nexum_apdu_transport_pcsc::PcscDeviceManager;
use std::error::Error;
use std::path::PathBuf;

// Re-export command handlers
mod card_operations;
mod credentials;
mod data_management;
mod key_operations;

// Re-export all command handlers
pub use card_operations::*;
pub use credentials::*;
pub use data_management::*;
pub use key_operations::*;

/// Define subcommands for the CLI
#[derive(Subcommand)]
pub enum Commands {
    /// List available readers
    List,

    /// Select the Keycard application and show info
    Select,

    /// Initialize a new card with random secrets
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

        /// Optional output file to save pairing info
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Pair with a Keycard
    Pair {
        /// Optional output file to save pairing info
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate a new key pair on the card
    GenerateKey {
        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Export the current key from the card
    ExportKey {
        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,

        /// Derivation path arguments
        #[command(flatten)]
        derivation: crate::utils::DerivationArgs,

        /// Export option (what parts of the key to export)
        #[arg(long = "export-option", value_enum, default_value_t = nexum_keycard::ExportOption::PublicKeyOnly)]
        export_option: nexum_keycard::ExportOption,
    },

    /// Sign data with the current key
    Sign {
        /// Data to sign (hex format)
        #[arg(required = true)]
        data: String,

        /// Derivation path arguments
        #[command(flatten)]
        derivation: crate::utils::DerivationArgs,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Change PIN, PUK, or pairing secret
    ChangeCredential {
        /// Type of credential to change: 'pin', 'puk', or 'pairing'
        #[arg(long, value_parser = ["pin", "puk", "pairing"])]
        credential_type: String,

        /// New value for the credential
        #[arg(long, required = true)]
        new_value: String,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Unblock PIN using PUK
    UnblockPin {
        #[arg(required = true)]
        puk: String,

        /// New PIN to set
        #[arg(required = true)]
        new_pin: String,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Set a PIN-less path for signature operations
    SetPinlessPath {
        /// Derivation path (e.g. m/44'/60'/0'/0/0) - required for this command
        #[arg(long, required = true)]
        path: String,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Load an existing key onto the card
    LoadKey {
        /// BIP39 seed or private key in hex format
        #[arg(required = true)]
        seed: String,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Store arbitrary data on the card
    StoreData {
        /// Data to store
        #[arg(required = true)]
        data: String,

        /// Record type to use
        #[arg(long = "record-type", value_enum, default_value_t = nexum_keycard::PersistentRecord::Public)]
        record_type: nexum_keycard::PersistentRecord,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Retrieve data from the card
    GetData {
        /// Record type to retrieve
        #[arg(long = "record-type", value_enum, default_value_t = nexum_keycard::PersistentRecord::Public)]
        record_type: nexum_keycard::PersistentRecord,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Remove the current key from the card
    RemoveKey {
        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Get detailed status information
    GetStatus {
        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Unpair from the card
    Unpair {
        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Generate a BIP39 mnemonic phrase on the card
    GenerateMnemonic {
        /// Number of words (12, 15, 18, 21, or 24)
        #[arg(long, default_value = "24", value_parser = clap::builder::ValueParser::new(|s: &str| -> Result<u8, String> {
            let val = s.parse::<u8>().map_err(|_| "Not a valid number".to_string())?;
            if val == 12 || val == 15 || val == 18 || val == 21 || val == 24 {
                Ok(val)
            } else {
                Err("Number of words must be 12, 15, 18, 21, or 24".to_string())
            }
        }))]
        words_count: u8,

        /// Pairing info for secure channel
        #[command(flatten)]
        pairing: crate::utils::PairingArgs,
    },

    /// Factory reset the card
    FactoryReset,
}

/// List all available readers
pub fn list_readers(manager: &PcscDeviceManager) -> Result<(), Box<dyn Error>> {
    crate::utils::reader::list_readers(manager)
}
