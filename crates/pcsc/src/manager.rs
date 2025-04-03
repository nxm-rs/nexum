//! Device manager for PC/SC operations

use pcsc::{Context, Scope};

use crate::config::{ConnectStrategy, PcscConfig};
use crate::error::PcscError;
use crate::monitor::PcscMonitor;
use crate::reader::PcscReader;
use crate::transport::PcscTransport;
use crate::util::match_atr;

/// Manager for PC/SC device operations
#[allow(missing_debug_implementations)]
pub struct PcscDeviceManager {
    /// PC/SC context
    context: Context,
}

// Implementation for standard library environments
impl PcscDeviceManager {
    /// Create a new PC/SC device manager
    pub fn new() -> Result<Self, PcscError> {
        let context = Context::establish(Scope::User)?;
        Ok(Self { context })
    }

    /// List all available card readers
    pub fn list_readers(&self) -> Result<Vec<PcscReader>, PcscError> {
        let readers = self.context.list_readers_owned()?;
        if readers.is_empty() {
            return Err(PcscError::NoReadersAvailable);
        }

        // For each reader, check if a card is present
        let mut result = Vec::with_capacity(readers.len());

        for reader_name in readers {
            // Create reader state to check for card presence
            let mut reader_states = vec![pcsc::ReaderState::new(
                reader_name.as_c_str(),
                pcsc::State::UNAWARE,
            )];

            // Get current state
            match self.context.get_status_change(None, &mut reader_states) {
                Ok(()) => {
                    let reader_state = &reader_states[0];
                    result.push(PcscReader::from_reader_state(reader_state));
                }
                Err(_) => {
                    // If we can't get status, assume no card
                    result.push(PcscReader::new(
                        reader_name.to_string_lossy().into_owned(),
                        false,
                        None,
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Open a connection to a specific reader
    pub fn open_reader(&self, reader_name: &str) -> Result<PcscTransport, PcscError> {
        self.open_reader_with_config(reader_name, PcscConfig::default())
    }

    /// Open a connection to a specific reader with custom configuration
    pub fn open_reader_with_config(
        &self,
        reader_name: &str,
        config: PcscConfig,
    ) -> Result<PcscTransport, PcscError> {
        // Clone the context to provide ownership to the transport
        let context = self.context.clone();
        PcscTransport::new(context, reader_name, config)
    }

    /// Connect to a reader using the specified strategy
    pub fn connect_strategy(
        &self,
        strategy: ConnectStrategy,
        config: PcscConfig,
    ) -> Result<PcscTransport, PcscError> {
        match strategy {
            ConnectStrategy::Reader(name) => self.open_reader_with_config(&name, config),
            ConnectStrategy::AnyCard => {
                // Find first reader with a card
                let readers = self.list_readers()?;
                for reader in readers {
                    if reader.has_card() {
                        return self.open_reader_with_config(reader.name(), config);
                    }
                }
                Err(PcscError::NoCard("No reader with card found".to_string()))
            }
            ConnectStrategy::CardWithAtr(pattern, mask) => {
                // Find reader with a card matching the ATR pattern
                let readers = self.list_readers()?;
                for reader in readers {
                    if let Some(atr) = reader.atr() {
                        if match_atr(atr, &pattern, mask.as_deref()) {
                            return self.open_reader_with_config(reader.name(), config);
                        }
                    }
                }
                Err(PcscError::NoCard(
                    "No card with matching ATR found".to_string(),
                ))
            }
            ConnectStrategy::FirstAvailable => {
                // Use first available reader
                let readers = self.list_readers()?;
                if readers.is_empty() {
                    return Err(PcscError::NoReadersAvailable);
                }
                self.open_reader_with_config(readers[0].name(), config)
            }
        }
    }

    /// Create a monitor for PC/SC events
    pub fn monitor(&self) -> Result<PcscMonitor, PcscError> {
        // Create a new context for the monitor to avoid conflicts
        let context = self.context.clone();
        PcscMonitor::new(context)
    }
}
