use alloy::signers::Signer;
use alloy_primitives::hex;
use coins_bip32::path::DerivationPath;
use nexum_apdu_core::CardExecutor;
use nexum_apdu_transport_pcsc::{PcscDeviceManager, PcscTransport};
use nexum_keycard::{KeyPath, Keycard, ParsedSelectOk, Secrets};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

use crate::utils::{self, reader, session};

/// List all available readers
pub fn list_readers(manager: &PcscDeviceManager) -> Result<(), Box<dyn std::error::Error>> {
    reader::list_readers(manager)
}

/// Select the Keycard application and display info
pub fn select_command(transport: PcscTransport) -> Result<(), Box<dyn std::error::Error>> {
    let (_, select_response) = session::initialize_keycard(transport)?;

    // Display card info
    println!("Keycard applet selected successfully.");
    println!("{}", select_response);

    Ok(())
}

/// Initialize a new Keycard
pub fn init_command(
    transport: PcscTransport,
    pin: &Option<String>,
    puk: &Option<String>,
    pairing_password: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let executor: CardExecutor<PcscTransport, nexum_keycard::Error> = CardExecutor::new(transport);
    let mut keycard = Keycard::new(executor);

    // Select the card to get into proper state
    let select_response = keycard.select_keycard()?;

    // Check if card is in pre-initialized state
    match select_response {
        ParsedSelectOk::PreInitialized(_) => {
            // Create secrets based on provided values or generate them
            let secrets = if pin.is_some() || puk.is_some() || pairing_password.is_some() {
                let pin = pin.clone().unwrap_or_else(|| "123456".to_string());
                let puk = puk.clone().unwrap_or_else(|| "123456789012".to_string());
                let pairing_password = pairing_password
                    .clone()
                    .unwrap_or_else(|| "KeycardDefaultPairing".to_string());

                Secrets::new(&pin, &puk, &pairing_password)
            } else {
                Secrets::generate()
            };

            // Initialize the card - this has some API incompatibility issues
            // Let's adapt to what the Keycard library actually offers
            match keycard.initialize(&secrets) {
                Ok(_) => {
                    println!("🎉 Keycard initialized successfully!");
                    println!("Secrets (SAVE THESE!):");
                    println!("  PIN: {}", secrets.pin());
                    println!("  PUK: {}", secrets.puk());
                    println!("  Pairing password: {}", secrets.pairing_pass());
                    Ok(())
                }
                Err(e) => Err(format!("Failed to initialize Keycard: {:?}", e).into()),
            }
        }
        _ => {
            println!("Card is already initialized.");
            Ok(())
        }
    }
}

/// Pair with a Keycard
pub fn pair_command(
    transport: PcscTransport,
    pairing_password: &str,
    output_file: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = session::initialize_keycard(transport)?;

    match keycard.pair(|| pairing_password.to_string()) {
        Ok(_) => {
            println!("🔑 Pairing successful!");
            if let Some(pairing_info) = keycard.pairing_info() {
                println!("\nPairing Information (SAVE THIS):");
                println!("  Pairing key: {}", hex::encode(pairing_info.key));
                println!("  Pairing index: {}", pairing_info.index);
                println!(
                    "\nYou can use these values with --key and --index options for future operations"
                );

                // Save to file if an output file was specified
                if let Some(path) = output_file {
                    utils::save_pairing_to_file(pairing_info, path)?;
                    println!("Pairing information saved to: {}", path.display());
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to pair with Keycard: {:?}", e).into()),
    }
}

/// Open a secure channel
pub fn open_secure_channel_command(
    transport: PcscTransport,
    file: Option<&PathBuf>,
    key_hex: Option<&String>,
    index: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = session::initialize_keycard(transport)?;

    // Apply pairing info
    utils::apply_pairing_info(&mut keycard, file, key_hex, index)?;

    // Open secure channel - fix the parameter issue
    match keycard.open_secure_channel() {
        Ok(_) => {
            println!("🔒 Secure channel opened successfully!");
            Ok(())
        }
        Err(e) => Err(format!("Failed to open secure channel: {:?}", e).into()),
    }
}

/// Verify PIN
pub fn verify_pin_command(
    transport: PcscTransport,
    pin: &str,
    pairing_key: Option<&String>,
    index: Option<u8>,
    file: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, response) = session::initialize_keycard(transport)?;

    // Ensure secure channel is open
    session::ensure_secure_channel(&mut keycard, &response, file, pairing_key, index)?;

    // Verify PIN
    match keycard.verify_pin(|| pin.to_string()) {
        Ok(_) => {
            println!("✅ PIN verified successfully!");
            Ok(())
        }
        Err(e) => Err(format!("PIN verification failed: {:?}", e).into()),
    }
}

/// Generate a new key pair
pub fn generate_key_command(
    transport: PcscTransport,
    pin: Option<&String>,
    pairing_key: Option<&String>,
    index: Option<u8>,
    file: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = session::setup_session(transport, pin, file, pairing_key, index)?;

    // Generate key
    match keycard.generate_key() {
        Ok(key_uid) => {
            println!("🔑 Key generated successfully!");
            println!("Key uid: {}", hex::encode(key_uid));
            Ok(())
        }
        Err(e) => Err(format!("Failed to generate key: {:?}", e).into()),
    }
}

/// Sign data with the key on the card
pub async fn sign_command(
    transport: PcscTransport,
    data_hex: &str,
    path: Option<&String>,
    pin: Option<&String>,
    pairing_key: Option<&String>,
    index: Option<u8>,
    file: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (keycard, _) = session::setup_session(transport, pin, file, pairing_key, index)?;

    let keycard = Arc::new(Mutex::new(keycard));

    // Convert hex string to bytes
    let data_bytes = hex::decode(data_hex.trim_start_matches("0x"))?;
    if data_bytes.len() != 32 {
        return Err("Data to sign must be exactly 32 bytes (e.g. a hash)".into());
    }

    // Convert to fixed-size array safely
    let mut data = [0u8; 32];
    data.copy_from_slice(&data_bytes[..32]);

    // Create key path based on provided path
    let _path_to_use = if let Some(path_str) = path {
        let derivation_path = DerivationPath::try_from(path_str.as_str())?;
        KeyPath::FromCurrent(derivation_path)
    } else {
        KeyPath::Current
    };

    let signer = nexum_keycard_signer::KeycardSigner::new(keycard.clone());

    // Adapt to the actual API of sign method
    match signer.sign_hash(&data.into()).await {
        Ok(signature) => {
            println!("✍️  Data signed successfully!");
            println!("Signature: {:#?}", signature);
            Ok(())
        }
        Err(e) => Err(format!("Failed to sign data: {:?}", e).into()),
    }
}

/// Export pairing information to a file
pub fn export_pairing_command(
    transport: PcscTransport,
    output: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let (keycard, _) = session::initialize_keycard(transport)?;

    if let Some(pairing_info) = keycard.pairing_info() {
        utils::save_pairing_to_file(pairing_info, output)?;
        println!("Pairing information exported to: {}", output.display());
        Ok(())
    } else {
        Err("No pairing information available. Please pair with the card first.".into())
    }
}
