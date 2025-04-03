//! Tests for the PcscTransport implementation

mod tests {
    use apdu_core::Bytes;
    use apdu_core::transport::CardTransport;
    use apdu_transport_pcsc::{PcscConfig, PcscDeviceManager};

    mod common;

    #[test]
    fn test_transport_creation() {
        let manager = match PcscDeviceManager::new() {
            Ok(manager) => manager,
            Err(_) => {
                println!("Skipping test, PC/SC not available");
                return;
            }
        };

        // Try to list readers
        match manager.list_readers() {
            Ok(readers) => {
                assert!(readers.len() > 0, "Expected at least one reader");

                // If we have a reader, try to create a transport
                if readers[0].has_card() {
                    let config = PcscConfig::default();
                    match manager.open_reader_with_config(readers[0].name(), config) {
                        Ok(transport) => {
                            assert!(
                                transport.is_connected(),
                                "Expected transport to be connected"
                            );
                        }
                        Err(e) => {
                            println!("Could not open reader {}: {:?}", readers[0].name(), e);
                        }
                    }
                } else {
                    println!("Skipping connection test, no card in reader");
                }
            }
            Err(e) => {
                println!("Could not list readers: {:?}", e);
            }
        }
    }

    #[test]
    fn test_transport_transmit() {
        // This test requires a real smartcard
        let transport = match common::get_test_transport() {
            Some(transport) => transport,
            None => {
                println!("Skipping test, no card available");
                return;
            }
        };

        let mut transport = transport;

        // Try to send a SELECT command (will work on most cards)
        let select_cmd = [0x00, 0xA4, 0x04, 0x00, 0x00]; // SELECT with empty AID
        match transport.transmit_raw(&select_cmd) {
            Ok(response) => {
                // We should get at least a 2-byte status response
                assert!(response.len() >= 2, "Response too short");

                // Print the response for debugging
                println!("Response: {}", hex::encode_upper(&response));
            }
            Err(e) => {
                println!("Transmit failed (might be expected): {:?}", e);
            }
        }
    }

    #[test]
    fn test_transport_reset() {
        // This test requires a real smartcard
        let transport = match common::get_test_transport() {
            Some(transport) => transport,
            None => {
                println!("Skipping test, no card available");
                return;
            }
        };

        let mut transport = transport;

        // Try to reset the card
        match transport.reset() {
            Ok(()) => {
                assert!(
                    transport.is_connected(),
                    "Transport should still be connected after reset"
                );
            }
            Err(e) => {
                println!("Reset failed (might be expected): {:?}", e);
            }
        }
    }
}
