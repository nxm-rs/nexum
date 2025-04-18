//! Commands for key management operations

use alloy_primitives::Address;
use alloy_primitives::hex::{self, ToHexExt};
use coins_bip32::path::DerivationPath;
use nexum_apdu_transport_pcsc::PcscTransport;
use nexum_keycard::ExportOption;
use std::error::Error;
use std::str::FromStr;
use tracing::info;

use crate::utils;

/// Generate a key on the card
pub fn generate_key_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Generate a new key
    info!("Generating master key");
    let key_uid = keycard.generate_key(true)?;

    println!("Key generated successfully");
    println!("Key UID: 0x{}", hex::encode(key_uid));

    Ok(())
}

/// Export the current key
pub fn export_key_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
    derivation_args: &utils::DerivationArgs,
    export_option: ExportOption,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Parse the derivation path
    let path = derivation_args.parse_derivation_path()?;
    info!("Exporting key with path: {}", derivation_args.path_string());

    // Export the key
    let keypair = keycard.export_key(export_option, &path)?;

    // Display the key information
    println!(
        "Key at path {} exported successfully",
        path.derivation_string()
    );

    // Display public key if available
    if let Some(public_key) = keypair.public_key() {
        println!(
            "Public key: 0x{}",
            hex::encode(public_key.to_sec1_bytes().as_ref())
        );
        println!(
            "Ethereum address: {}",
            Address::from_public_key(&public_key.into())
        );
    }

    // Display private key if available
    if let Some(private_key) = keypair.private_key() {
        println!("Private key: 0x{}", hex::encode(private_key.to_bytes()));
    }

    // Display chain code if available
    if let Some(chain_code) = keypair.chain_code() {
        println!("Chain code: 0x{}", hex::encode(chain_code));
    }

    Ok(())
}

/// Sign data with the current key
pub async fn sign_command(
    transport: PcscTransport,
    data: &str,
    derivation_args: &utils::DerivationArgs,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Parse the data from hex
    let data_bytes = hex::decode(data)?;

    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Parse the derivation path
    let derivation_path = derivation_args.parse_derivation_path()?;
    info!(
        "Signing with key at path: {}",
        derivation_args.path_string()
    );

    // The actual path derivation is handled internally by the keycard
    let signature = keycard.sign(&data_bytes, &derivation_path, true)?;

    // Display the signature
    println!(
        "Signature: {}",
        signature.as_bytes().encode_hex_with_prefix()
    );

    Ok(())
}

/// Load an existing key
pub fn load_key_command(
    transport: PcscTransport,
    seed: &str,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Check if the seed looks like a hex string and decode it
    let seed_bytes = if seed.len() >= 2 && seed.starts_with("0x") {
        hex::decode(&seed[2..])?
    } else if seed.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(seed)?
    } else {
        // We assume it's a mnemonic phrase, but we need to do a manual conversion
        // since we don't have direct access to BIP39 from here
        return Err("Mnemonic phrases are not supported yet. Please use hex seed instead.".into());
    };

    // Load the key from seed
    keycard.load_seed(&seed_bytes.try_into().unwrap(), true)?;

    println!("Key loaded successfully");

    Ok(())
}

/// Remove the current key
pub fn remove_key_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Remove the key
    keycard.remove_key(true)?;

    println!("Key removed successfully");

    Ok(())
}

/// Set a PIN-less path for signature operations
pub fn set_pinless_path_command(
    transport: PcscTransport,
    path: &str,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Parse the derivation path
    let derivation_path = DerivationPath::from_str(path)?;

    // Set the PIN-less path
    keycard.set_pinless_path(Some(&derivation_path), false)?;

    println!("PIN-less path set to: {}", path);

    Ok(())
}

/// Generate a BIP39 mnemonic on the card
pub fn generate_mnemonic_command(
    transport: PcscTransport,
    words_count: u8,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Generate mnemonic
    let mnemonic = keycard.generate_mnemonic(words_count)?;

    println!("Generated {} word mnemonic:", words_count);
    println!("{}", mnemonic.to_phrase());

    Ok(())
}
