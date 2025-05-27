//! Session management for the Keycard CLI

use nexum_apdu_core::prelude::*;
use nexum_apdu_transport_pcsc::PcscTransport;
use nexum_keycard::{Keycard, KeycardSecureChannel, PairingInfo};
use tracing::debug;

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

/// Initialize a keycard with pairing information
pub fn initialize_keycard(
    transport: PcscTransport,
    pairing_args: Option<&crate::utils::PairingArgs>,
) -> Result<Keycard<KeycardExecutor>, Box<dyn std::error::Error>> {
    // Create input and confirmation callbacks
    let input_callback = Box::new(default_input_request);
    let confirmation_callback = Box::new(default_confirmation);

    // If we have pairing information, try to load and establish a secure channel
    let pairing_info = match pairing_args {
        Some(args) => get_pairing_info(args)?,
        None => None,
    };

    // Create a new keycard with the executor
    let keycard = Keycard::from_interactive(
        transport,
        input_callback,
        confirmation_callback,
        None,
        pairing_info,
    )?;

    Ok(keycard)
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
