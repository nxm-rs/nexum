use alloy_primitives::hex;
use clap::Args;
use nexum_apdu_core::{ApduExecutorErrors, SecureChannelExecutor};
use nexum_keycard::{Error, PairingInfo};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

pub mod reader;
pub mod session;

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

/// Load pairing information from a file
pub fn load_pairing_from_file(path: &PathBuf) -> Result<PairingInfo, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Parse format: index,key_hex
    let parts: Vec<&str> = content.trim().split(',').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid pairing file format. Expected 'index,key_hex' but got: {}",
            content
        )
        .into());
    }

    let index = parts[0].parse::<u8>()?;
    let key: [u8; 32] = hex::decode(parts[1])?.try_into().map_err(|_| {
        format!(
            "Invalid key length. Expected 32 bytes but got {}",
            parts[1].len()
        )
    })?;

    Ok(PairingInfo {
        key: key.into(),
        index,
    })
}

/// Save pairing information to a file
pub fn save_pairing_to_file(
    pairing_info: &PairingInfo,
    path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path)?;

    // Format: index,key_hex
    let content = format!("{},{}", pairing_info.index, hex::encode(pairing_info.key));
    file.write_all(content.as_bytes())?;

    Ok(())
}

/// Prompt for PIN
pub fn prompt_for_pin() -> Result<String, Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    print!("Enter PIN: ");
    io::stdout().flush()?;
    let mut pin = String::new();
    io::stdin().read_line(&mut pin)?;
    Ok(pin.trim().to_string())
}

/// Apply pairing information to a Keycard instance
pub fn apply_pairing_info<E>(
    keycard: &mut nexum_keycard::Keycard<E>,
    file: Option<&PathBuf>,
    key_hex: Option<&String>,
    index: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>>
where
    E: SecureChannelExecutor,
    Error: From<<E as ApduExecutorErrors>::Error>,
{
    // Set pairing info - either from file or from key and index
    if let Some(file_path) = file {
        // Load pairing info from file
        let pairing_info = load_pairing_from_file(file_path)?;
        keycard.set_pairing_info(pairing_info);
        Ok(())
    } else if let (Some(key_hex), Some(idx)) = (key_hex, index) {
        // Use provided key and index
        let pairing_key: [u8; 32] = hex::decode(key_hex.trim_start_matches("0x"))?
            .try_into()
            .unwrap();
        let pairing_info = PairingInfo {
            key: pairing_key.into(),
            index: idx,
        };
        keycard.set_pairing_info(pairing_info);
        Ok(())
    } else {
        Err("No pairing information provided. Use --file or --key with --index.".into())
    }
}
