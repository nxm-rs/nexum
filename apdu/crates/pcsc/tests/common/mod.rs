//! Common test utilities

use pcsc::{Context, Scope};

use apdu_transport_pcsc::{PcscConfig, PcscDeviceManager, PcscTransport};

/// Try to get a real PC/SC context for tests
pub fn get_pcsc_context() -> Option<Context> {
    match Context::establish(Scope::User) {
        Ok(context) => Some(context),
        Err(_) => None,
    }
}

/// Try to get a real reader with a card for tests
pub fn get_reader_with_card() -> Option<String> {
    let context = get_pcsc_context()?;
    let readers = context.list_readers_owned().ok()?;

    // Use first reader for tests
    if !readers.is_empty() {
        return Some(readers[0].to_string_lossy().into_owned());
    }

    None
}

/// Try to get a real transport for tests
pub fn get_test_transport() -> Option<PcscTransport> {
    let manager = PcscDeviceManager::new().ok()?;
    let reader_name = get_reader_with_card()?;
    let config = PcscConfig::default();

    manager.open_reader_with_config(&reader_name, config).ok()
}
