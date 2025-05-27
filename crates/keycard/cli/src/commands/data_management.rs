//! Commands for data storage and retrieval operations

use alloy_primitives::hex;
use nexum_apdu_transport_pcsc::PcscTransport;
use nexum_keycard::PersistentRecord;
use std::error::Error;

use crate::utils;

/// Store data on the card
pub fn store_data_command(
    transport: PcscTransport,
    record: PersistentRecord,
    data: &[u8],
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Store the data with the provided record type
    let record_label = format!("{:?}", record);
    keycard.store_data(record, data)?;

    println!(
        "Data stored successfully using {} record type",
        record_label
    );

    Ok(())
}

/// Retrieve data from the card
pub fn get_data_command(
    transport: PcscTransport,
    record: PersistentRecord,
    pairing_args: &utils::PairingArgs,
) -> Result<(), Box<dyn Error>> {
    // Initialize keycard with pairing info
    let mut keycard = utils::session::initialize_keycard(transport, Some(pairing_args))?;

    // Get the data by record type
    let record_label = format!("{:?}", record);
    let data = keycard.get_data(record)?;

    println!(
        "Retrieved data from {} record (length: {} bytes): {}",
        record_label,
        data.len(),
        hex::encode(&data)
    );

    // Try to interpret as UTF-8 string if possible
    if let Ok(str_data) = std::str::from_utf8(&data) {
        if str_data
            .chars()
            .all(|c| !c.is_control() || c == '\n' || c == '\t' || c == '\r')
        {
            println!("Data as string: {}", str_data);
        }
    }

    Ok(())
}
