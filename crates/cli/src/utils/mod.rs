//! Utility functions and types for the Keycard CLI

pub mod reader;
pub mod session;

use alloy_primitives::hex;
use clap::Args;
use coins_bip32::path::DerivationPath;
use nexum_keycard::PairingInfo;
use rand::Rng;
use rand::distr::Alphanumeric;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

/// Arguments for derivation path
#[derive(Args, Debug, Clone)]
pub struct DerivationArgs {
    /// Derivation path (e.g. m/44'/60'/0'/0/0)
    #[arg(long, default_value = "m/44'/60'/0'/0/0")]
    pub path: String,
}

impl DerivationArgs {
    /// Parse the derivation path
    pub fn parse_derivation_path(&self) -> Result<DerivationPath, Box<dyn Error>> {
        let path = DerivationPath::from_str(&self.path)?;
        Ok(path)
    }

    /// Get the path string representation
    pub fn path_string(&self) -> &str {
        &self.path
    }
}

/// Common arguments for pairing information
#[derive(Args, Debug, Clone)]
pub struct PairingArgs {
    /// Path to file containing pairing data
    #[arg(long, group = "pairing")]
    pub file: Option<PathBuf>,

    /// Pairing key in hex (must be used with --index)
    #[arg(long, requires = "index", group = "pairing")]
    pub key: Option<String>,

    /// Pairing index (must be used with --key)
    #[arg(long, requires = "key")]
    pub index: Option<u8>,
}

/// Save pairing information to a file
pub fn save_pairing_to_file(
    pairing_info: &PairingInfo,
    path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(path)?;

    // Format: index,key_hex
    let content = format!(
        "{},{}",
        pairing_info.index,
        hex::encode(pairing_info.key.as_slice())
    );
    file.write_all(content.as_bytes())?;

    Ok(())
}

/// Load pairing information from a file
pub fn load_pairing_from_file(path: &PathBuf) -> Result<PairingInfo, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let parts: Vec<&str> = content.trim().split(',').collect();
    if parts.len() != 2 {
        return Err("Invalid pairing file format".into());
    }

    let index = parts[0].parse::<u8>()?;
    let key = hex::decode(parts[1])?;

    // Create a new PairingInfo instance with the key and index
    Ok(PairingInfo {
        key: key.try_into().unwrap(),
        index,
    })
}

/// Generate a random PIN (6 digits)
pub fn generate_random_pin() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0..1000000))
}

/// Generate a random PUK (12 digits)
pub fn generate_random_puk() -> String {
    let mut rng = rand::rng();
    format!("{:012}", rng.random_range(0..1000000000000u64))
}

/// Generate a random pairing password (UTF-8 string)
pub fn generate_random_pairing_password() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect()
}
