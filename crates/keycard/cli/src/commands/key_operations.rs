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
    use crate::utils::display;

    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Generate a new key
    info!("Generating master key");
    let key_uid = keycard.generate_key(true)?;

    println!("{}", display::success("Key generated successfully"));
    println!(
        "{}",
        display::key_value_box("Key Details", vec![(
            "UID",
            format!("0x{}", hex::encode(key_uid))
        )])
    );

    Ok(())
}

/// Export the current key
pub fn export_key_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
    derivation_args: &utils::DerivationArgs,
    export_option: ExportOption,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;

    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Parse the derivation path
    let path = derivation_args.parse_derivation_path()?;
    info!("Exporting key with path: {}", derivation_args.path_string());

    // Export the key
    let keypair = keycard.export_key(export_option, &path)?;

    println!(
        "{}",
        display::success(
            format!(
                "Key at path {} exported successfully",
                path.derivation_string()
            )
            .as_str()
        )
    );

    // Build our key value items
    let mut key_items = Vec::new();

    // Display public key if available
    if let Some(public_key) = keypair.public_key() {
        key_items.push((
            "Public key",
            format!("0x{}", hex::encode(public_key.to_sec1_bytes().as_ref())),
        ));
        key_items.push((
            "Ethereum address",
            Address::from_public_key(&public_key.into()).to_string(),
        ));
    }

    // Display private key if available
    if let Some(private_key) = keypair.private_key() {
        key_items.push((
            "Private key",
            format!("0x{}", hex::encode(private_key.to_bytes())),
        ));
    }

    // Display chain code if available
    if let Some(chain_code) = keypair.chain_code() {
        key_items.push(("Chain code", format!("0x{}", hex::encode(chain_code))));
    }

    // Convert key_items to proper types
    let fixed_items: Vec<(&str, String)> = key_items
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();

    // Display all key information in a box
    println!("{}", display::key_value_box("Key Information", fixed_items));

    Ok(())
}

/// Sign data with the current key
pub async fn sign_command(
    transport: PcscTransport,
    data: &str,
    derivation_args: &utils::DerivationArgs,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;

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

    println!(
        "{}",
        display::success(
            format!(
                "Data signed successfully with key at {}",
                derivation_args.path_string()
            )
            .as_str()
        )
    );

    // Display the signature in a box format
    println!(
        "{}",
        display::key_value_box("Signature", vec![(
            "Value",
            signature.as_bytes().encode_hex_with_prefix().to_string()
        )])
    );

    Ok(())
}

/// Load an existing key
pub fn load_seed_command(
    transport: PcscTransport,
    password: bool,
    language: &str,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Handle seed phrase input
    // Import necessary components for BIP39
    use coins_bip39::Mnemonic;
    use coins_bip39::{
        ChineseSimplified, ChineseTraditional, Czech, English, French, Italian, Japanese, Korean,
        Portuguese, Spanish,
    };

    // Get mnemonic phrase from user using the utility function
    let mnemonic_phrase = utils::session::default_input_request("Enter your seed phrase");

    // Get password if requested
    let password = if password {
        // User specified the --password flag, so prompt for it
        Some(utils::session::default_input_request(
            "Enter password for seed phrase",
        ))
    } else {
        None
    };

    // Use a generic function to handle different language wordlists
    fn parse_and_load_seed<L>(
        phrase: &str,
        password: Option<String>,
        keycard: &mut nexum_keycard::Keycard<
            nexum_apdu_core::prelude::CardExecutor<
                nexum_keycard::KeycardSecureChannel<nexum_apdu_transport_pcsc::PcscTransport>,
            >,
        >,
    ) -> Result<[u8; 32], Box<dyn Error>>
    where
        L: coins_bip39::Wordlist,
    {
        // Parse the mnemonic phrase
        let mnemonic = Mnemonic::<L>::new_from_phrase(phrase)
            .map_err(|e| format!("Failed to parse mnemonic: {}", e))?;

        // Load the key from seed
        Ok(match password {
            Some(p) => keycard.load_seed(&mnemonic.to_seed(Some(&p))?, true)?,
            None => keycard.load_seed(&mnemonic.to_seed(None)?, true)?,
        })
    }

    // Call the appropriate function based on the selected language
    let result = match language {
        "english" => parse_and_load_seed::<English>(&mnemonic_phrase, password, &mut keycard),
        "chinese_simplified" => {
            parse_and_load_seed::<ChineseSimplified>(&mnemonic_phrase, password, &mut keycard)
        }
        "chinese_traditional" => {
            parse_and_load_seed::<ChineseTraditional>(&mnemonic_phrase, password, &mut keycard)
        }
        "czech" => parse_and_load_seed::<Czech>(&mnemonic_phrase, password, &mut keycard),
        "french" => parse_and_load_seed::<French>(&mnemonic_phrase, password, &mut keycard),
        "italian" => parse_and_load_seed::<Italian>(&mnemonic_phrase, password, &mut keycard),
        "japanese" => parse_and_load_seed::<Japanese>(&mnemonic_phrase, password, &mut keycard),
        "korean" => parse_and_load_seed::<Korean>(&mnemonic_phrase, password, &mut keycard),
        "portuguese" => parse_and_load_seed::<Portuguese>(&mnemonic_phrase, password, &mut keycard),
        "spanish" => parse_and_load_seed::<Spanish>(&mnemonic_phrase, password, &mut keycard),
        _ => return Err(format!("Unsupported language: {}", language).into()),
    }?;

    // Handle the result
    use crate::utils::display;

    println!(
        "{}",
        display::success("Key loaded successfully from seed phrase")
    );
    println!(
        "{}",
        display::key_value_box("Key Details", vec![(
            "UID",
            result.encode_hex_with_prefix()
        )])
    );

    Ok(())
}

/// Remove the current key
pub fn remove_key_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;

    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Remove the key
    keycard.remove_key(true)?;

    println!("{}", display::success("Key removed successfully"));
    println!("{}", display::info("The card no longer has a key loaded"));

    Ok(())
}

/// Set a PIN-less path for signature operations
pub fn set_pinless_path_command(
    transport: PcscTransport,
    path: &str,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;

    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Parse the derivation path
    let derivation_path = DerivationPath::from_str(path)?;

    // Set the PIN-less path
    keycard.set_pinless_path(Some(&derivation_path), false)?;

    println!("{}", display::success("PIN-less path set successfully"));
    println!(
        "{}",
        display::key_value_box("PIN-less Path", vec![("Path", path.to_string())])
    );
    println!(
        "{}",
        display::info("This path can now be used for signing without requiring a PIN")
    );

    Ok(())
}

/// Generate a BIP39 mnemonic on the card
pub fn generate_mnemonic_command(
    transport: PcscTransport,
    words_count: u8,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;

    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Generate mnemonic
    let mnemonic = keycard.generate_mnemonic(words_count)?;

    println!(
        "{}",
        display::success(format!("Generated {} word mnemonic", words_count).as_str())
    );
    println!("{}", display::sensitive_data_warning());

    // Display the mnemonic in a key value box
    println!(
        "{}",
        display::key_value_box("MNEMONIC PHRASE", vec![("Phrase", mnemonic.to_phrase())])
    );

    Ok(())
}
