use crate::utils::PairingArgs;
use nexum_apdu_transport_pcsc::PcscTransport;

/// Change credentials (PIN, PUK, or pairing secret)
pub fn change_credentials_command(
    transport: PcscTransport,
    credential_type: &str,
    new_value: &str,
    pin: Option<&String>,
    pairing: &PairingArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = crate::utils::session::setup_session(
        transport,
        pin,
        pairing.file.as_ref(),
        pairing.key.as_ref(),
        pairing.index,
    )?;

    // Ensure PIN is verified
    crate::utils::session::ensure_pin_verified(&mut keycard, pin)?;

    // Call the change_credential method with adapted parameters
    match credential_type.to_lowercase().as_str() {
        "pin" => {
            // Validate PIN format
            if new_value.len() != 6 || !new_value.chars().all(|c| c.is_ascii_digit()) {
                return Err("PIN must be exactly 6 digits".into());
            }
            keycard.change_pin(new_value)?;
            println!("✅ PIN changed successfully!");
        }
        "puk" => {
            // Validate PUK format
            if new_value.len() != 12 || !new_value.chars().all(|c| c.is_ascii_digit()) {
                return Err("PUK must be exactly 12 digits".into());
            }
            keycard.change_puk(new_value)?;
            println!("✅ PUK changed successfully!");
        }
        "pairing" => {
            keycard.change_pairing_secret(new_value.as_bytes())?;
            println!("✅ Pairing secret changed successfully!");
        }
        _ => {
            return Err(format!(
                "Unknown credential type: {}. Use 'pin', 'puk', or 'pairing'",
                credential_type
            )
            .into());
        }
    }

    Ok(())
}

/// Unblock PIN using PUK
pub fn unblock_pin_command(
    transport: PcscTransport,
    puk: &str,
    new_pin: &str,
    pairing: &PairingArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = crate::utils::session::setup_session(
        transport,
        None,
        pairing.file.as_ref(),
        pairing.key.as_ref(),
        pairing.index,
    )?;

    // Validate new PIN format
    if new_pin.len() != 6 || !new_pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("New PIN must be exactly 6 digits".into());
    }

    // Validate PUK format
    if puk.len() != 12 || !puk.chars().all(|c| c.is_ascii_digit()) {
        return Err("PUK must be exactly 12 digits".into());
    }

    // Unblock PIN
    match keycard.unblock_pin(puk, new_pin) {
        Ok(_) => {
            println!("✅ PIN unblocked successfully!");
            Ok(())
        }
        Err(e) => Err(format!("Failed to unblock PIN: {:?}", e).into()),
    }
}

/// Set PIN-less path for signature operations
pub fn set_pinless_path_command(
    transport: PcscTransport,
    path_str: &str,
    pin: Option<&String>,
    pairing: &PairingArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = crate::utils::session::setup_session(
        transport,
        pin,
        pairing.file.as_ref(),
        pairing.key.as_ref(),
        pairing.index,
    )?;

    // Ensure PIN is verified
    crate::utils::session::ensure_pin_verified(&mut keycard, pin)?;

    // Adapt to whatever API the keycard library actually has
    match keycard.set_pinless_path(path_str) {
        Ok(_) => {
            println!("✅ PIN-less path set to: {}", path_str);
            println!("You can now sign using this path without PIN verification.");
            Ok(())
        }
        Err(e) => Err(format!("Failed to set PIN-less path: {:?}", e).into()),
    }
}

/// Remove the current key from the card
pub fn remove_key_command(
    transport: PcscTransport,
    pin: Option<&String>,
    pairing: &PairingArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, _) = crate::utils::session::setup_session(
        transport,
        pin,
        pairing.file.as_ref(),
        pairing.key.as_ref(),
        pairing.index,
    )?;

    // Ensure PIN is verified
    crate::utils::session::ensure_pin_verified(&mut keycard, pin)?;

    // Remove the current key
    match keycard.remove_key() {
        Ok(_) => {
            println!("🗑️ Key removed successfully from the card");
            Ok(())
        }
        Err(e) => Err(format!("Failed to remove key: {:?}", e).into()),
    }
}

/// Get detailed status information about the card
pub fn get_status_command(transport: PcscTransport) -> Result<(), Box<dyn std::error::Error>> {
    let (mut keycard, select_response) = crate::utils::session::initialize_keycard(transport)?;

    println!("Keycard application info:");
    println!("{}", select_response);

    // Try to get extended status
    match keycard.get_status() {
        Ok(status) => {
            println!("\nApplication Status:");
            println!("  PIN retries remaining: {}", status.pin_retry_count);
            println!("  PUK retries remaining: {}", status.puk_retry_count);
            println!(
                "  Key initialized: {}",
                if status.key_initialized { "Yes" } else { "No" }
            );
            Ok(())
        }
        Err(e) => {
            println!("Could not get detailed status: {:?}", e);
            Ok(()) // Not treating this as a fatal error
        }
    }
}
