//! Session management for the Keycard CLI

use nexum_apdu_core::prelude::*;
use nexum_apdu_transport_pcsc::PcscTransport;
use nexum_keycard::{ApplicationInfo, Keycard, KeycardSecureChannel, PairingInfo};
use tracing::{debug, error};

type KeycardExecutor = CardExecutor<KeycardSecureChannel<PcscTransport>>;

/// Default input request handler (asks for PIN/PUK/etc)
pub fn default_input_request(prompt: &str) -> String {
    use std::io::{self, Write};
    print!("{}: ", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

/// Default confirmation handler
pub fn default_confirmation(message: &str) -> bool {
    use std::io::{self, Write};
    print!("{} (y/n): ", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

/// Initialize a keycard from transport and select the application
pub fn initialize_keycard(
    transport: PcscTransport,
) -> Result<(Keycard<KeycardExecutor>, ApplicationInfo), Box<dyn std::error::Error>> {
    // Create a keycard secure channel around the transport
    let secure_channel = KeycardSecureChannel::new(transport);

    // Create a CardExecutor from the secure channel
    let card_executor = CardExecutor::new(secure_channel);

    // Create input and confirmation callbacks
    let input_callback = Box::new(default_input_request);
    let confirmation_callback = Box::new(default_confirmation);

    // Create a new keycard with the executor
    let mut keycard = Keycard::new(card_executor, input_callback, confirmation_callback)?;

    // Select the keycard application
    let app_info = keycard.select_keycard()?;

    Ok((keycard, app_info))
}

/// Initialize a keycard with pairing information
pub fn initialize_keycard_with_pairing(
    transport: PcscTransport,
    pairing_args: &crate::utils::PairingArgs,
) -> Result<(Keycard<KeycardExecutor>, Option<ApplicationInfo>), Box<dyn std::error::Error>> {
    // Create a keycard secure channel around the transport
    let secure_channel = KeycardSecureChannel::new(transport);

    // Create a CardExecutor from the secure channel
    let card_executor = CardExecutor::new(secure_channel);

    // Create input and confirmation callbacks
    let input_callback = Box::new(default_input_request);
    let confirmation_callback = Box::new(default_confirmation);

    // Create a new keycard with the executor
    let mut keycard = Keycard::new(card_executor, input_callback, confirmation_callback)?;

    // If we have pairing information, try to load and establish a secure channel
    let pairing_info = get_pairing_info(pairing_args)?;
    if let Some(info) = pairing_info {
        debug!("Using pairing info with index {}", info.index);
        keycard.set_pairing_info(info);
    }

    // Select the keycard application
    let app_info = keycard.select_keycard().ok();

    // If we have pairing info, try to open a secure channel
    if keycard.pairing_info().is_some() && app_info.is_some() {
        debug!("Opening secure channel");
        match keycard.open_secure_channel() {
            Ok(_) => debug!("Secure channel established successfully"),
            Err(e) => {
                error!("Failed to open secure channel: {:?}", e);
                // Continue without secure channel
            }
        }
    }

    Ok((keycard, app_info))
}

/// Extract pairing information from pairing arguments
pub fn get_pairing_info(
    pairing_args: &crate::utils::PairingArgs,
) -> Result<Option<PairingInfo>, Box<dyn std::error::Error>> {
    if let Some(file) = &pairing_args.file {
        debug!("Loading pairing info from file: {:?}", file);
        let pairing_info = crate::utils::load_pairing_from_file(file)?;
        return Ok(Some(pairing_info));
    } else if let (Some(key), Some(index)) = (&pairing_args.key, pairing_args.index) {
        debug!("Using pairing info from command line arguments");
        let key_bytes = alloy_primitives::hex::decode(key)?;
        // Create PairingInfo with the key and index
        return Ok(Some(PairingInfo {
            key: key_bytes.try_into().unwrap(),
            index,
        }));
    }

    Ok(None)
}
