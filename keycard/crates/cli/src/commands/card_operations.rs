//! Commands for basic card operations

use nexum_apdu_transport_pcsc::PcscTransport;
use nexum_keycard::Secrets;
use std::error::Error;
use std::path::PathBuf;
use tracing::{debug, info};

use crate::utils;

/// Select the Keycard application and display info
pub fn select_command(transport: PcscTransport) -> Result<(), Box<dyn Error>> {
    let mut keycard = utils::session::initialize_keycard(transport, None)?;
    let app_info = keycard.select_keycard()?;

    // Display card info
    info!("Keycard applet selected successfully.");
    println!("{}", app_info);

    Ok(())
}

/// Initialize a new Keycard
pub fn init_command(
    transport: PcscTransport,
    pin: &Option<String>,
    puk: &Option<String>,
    pairing_password: &Option<String>,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;
    
    // Create a keycard instance
    let mut keycard = utils::session::initialize_keycard(transport, None)?;

    // Create secrets based on provided values or generate them
    let secrets = if pin.is_some() || puk.is_some() || pairing_password.is_some() {
        let pin = pin.clone().unwrap_or_else(utils::generate_random_pin);
        let puk = puk.clone().unwrap_or_else(utils::generate_random_puk);
        let pairing_password = pairing_password
            .clone()
            .unwrap_or_else(utils::generate_random_pairing_password);

        debug!("Using provided secrets");
        Secrets::new(&pin, &puk, &pairing_password)
    } else {
        debug!("Generating random secrets");
        Secrets::generate_v3_1(3, 5, true)
    };

    // Initialize the card
    keycard.initialize(&secrets, true)?;

    println!("{}", display::success("Keycard initialized successfully!"));
    println!("{}", display::sensitive_data_warning());
    
    println!("{}", display::key_value_box("Security Credentials", vec![
        ("PIN", secrets.pin().to_string()),
        ("PUK", secrets.puk().to_string()),
        ("Pairing password", secrets.pairing_pass().to_string())
    ]));

    Ok(())
}

/// Pair with a card
pub fn pair_command(
    transport: PcscTransport,
    output_file: Option<&PathBuf>,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;
    
    info!("Pairing with card");

    // Create a keycard instance
    let mut keycard = utils::session::initialize_keycard(transport, None)?;

    // Perform the pairing
    let pairing_info = keycard.pair()?;

    println!("{}", display::success("Pairing successful!"));
    
    println!("{}", display::key_value_box("Pairing Information", vec![
        ("Index", pairing_info.index.to_string()),
        ("Key", alloy_primitives::hex::encode(pairing_info.key.as_slice()).to_string())
    ]));

    // Save pairing info to file if requested
    if let Some(path) = output_file {
        utils::save_pairing_to_file(&pairing_info, path)?;
        println!("{}", display::info(format!("Pairing information saved to {:?}", path).as_str()));
    }

    Ok(())
}

/// Unpair from a card
pub fn unpair_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;
    
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // We need pairing info to unpair
    if keycard.pairing_info().is_none() {
        return Err("Pairing information is required for unpair command".into());
    }

    // Unpair
    let index = keycard.pairing_info().unwrap().index;
    info!("Removing pairing with index {} from card", index);
    keycard.unpair(index, true)?;
    
    println!("{}", display::success("Pairing removed successfully"));
    println!("{}", display::info(format!("Pairing slot {} is now available", index).as_str()));

    Ok(())
}

/// Get detailed status information
pub fn get_status_command(
    transport: PcscTransport,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;
    
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Given that can get pairing information, we can fetch all the data
    let application_info = keycard.select_keycard()?;
    let application_status = keycard.get_status()?;

    // Display the information we have fetched
    println!("{}", display::section_title("Keycard Information"));
    println!("{}", application_info);
    
    println!("{}", display::section_title("Keycard Status"));
    println!("{}", application_status);

    Ok(())
}

/// Factory reset the card
pub fn factory_reset_command(transport: PcscTransport) -> Result<(), Box<dyn Error>> {
    use crate::utils::display;
    
    // Initialize keycard with no pairing info (no secure channel / pairing required for FACTORY RESET)
    let mut keycard = utils::session::initialize_keycard(transport, None)?;

    // Factory reset the card
    keycard.factory_reset(true)?;
    
    println!("{}", display::success("Card factory reset completed successfully"));
    println!("{}", display::info("The card has been restored to factory settings and needs to be initialized again"));

    Ok(())
}
