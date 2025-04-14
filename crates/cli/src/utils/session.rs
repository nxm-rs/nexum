use nexum_apdu_core::CardExecutor;
use nexum_apdu_transport_pcsc::PcscTransport;
use nexum_keycard::{Keycard, ParsedSelectOk};
use std::path::PathBuf;
use tracing::{debug, info};

use super::{apply_pairing_info, prompt_for_pin};

/// Initialize a Keycard session with a transport
pub fn initialize_keycard(
    transport: PcscTransport,
) -> Result<
    (
        Keycard<CardExecutor<PcscTransport, nexum_keycard::Error>>,
        ParsedSelectOk,
    ),
    Box<dyn std::error::Error>,
> {
    let executor = CardExecutor::new(transport);
    let mut keycard = Keycard::new(executor);

    // Select Keycard application
    info!("Selecting Keycard application...");
    let select_response = keycard.select_keycard()?;

    Ok((keycard, select_response))
}

/// Ensure a secure channel is established
pub fn ensure_secure_channel(
    keycard: &mut Keycard<CardExecutor<PcscTransport, nexum_keycard::Error>>,
    _response: &ParsedSelectOk,
    file: Option<&PathBuf>,
    key_hex: Option<&String>,
    index: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !keycard.is_secure_channel_open() {
        // Apply pairing info if provided
        apply_pairing_info(keycard, file, key_hex, index)?;

        // Open secure channel - adding a fix for the parameter issue
        debug!("Opening secure channel");
        // The function signature may have changed - we'll adapt our call
        keycard.open_secure_channel()?;
        info!("Secure channel opened successfully");
    }

    Ok(())
}

/// Ensure PIN is verified
pub fn ensure_pin_verified(
    keycard: &mut Keycard<CardExecutor<PcscTransport, nexum_keycard::Error>>,
    pin: Option<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !keycard.is_pin_verified() {
        let pin_to_use = match pin {
            Some(p) => p.clone(),
            None => prompt_for_pin()?,
        };

        debug!("Verifying PIN");
        keycard.verify_pin(|| pin_to_use)?;
        info!("PIN verified successfully");
    }

    Ok(())
}

/// Setup a Keycard session with secure channel and PIN verification
pub fn setup_session(
    transport: PcscTransport,
    pin: Option<&String>,
    file: Option<&PathBuf>,
    key_hex: Option<&String>,
    index: Option<u8>,
) -> Result<
    (
        Keycard<CardExecutor<PcscTransport, nexum_keycard::Error>>,
        ParsedSelectOk,
    ),
    Box<dyn std::error::Error>,
> {
    // Initialize Keycard
    let (mut keycard, response) = initialize_keycard(transport)?;

    // Ensure secure channel
    ensure_secure_channel(&mut keycard, &response, file, key_hex, index)?;

    // Verify PIN if provided
    if pin.is_some() {
        ensure_pin_verified(&mut keycard, pin)?;
    }

    Ok((keycard, response))
}
