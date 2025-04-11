//! PC/SC transport implementation

use nexum_apdu_core::prelude::*;

use pcsc::{Card, Context, Disposition};
use std::{ffi::CString, fmt};

use crate::{config::PcscConfig, error::PcscError};

/// Transport implementation using PC/SC
pub struct PcscTransport {
    /// PC/SC context
    context: Context,
    /// Card connection, if established
    card: Option<Card>,
    /// Reader name
    reader_name: String,
    /// Configuration
    config: PcscConfig,
    /// Whether a transaction is active
    transaction_active: bool,
}

impl fmt::Debug for PcscTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PcscTransport")
            .field("reader_name", &self.reader_name)
            .field("has_card", &self.card.is_some())
            .field("config", &self.config)
            .field("transaction_active", &self.transaction_active)
            .finish()
    }
}

// Implementation for standard library environments
impl PcscTransport {
    /// Create a new PC/SC transport for the specified reader
    pub(crate) fn new(
        context: Context,
        reader_name: &str,
        config: PcscConfig,
    ) -> Result<Self, PcscError> {
        let reader_name = reader_name.to_string();

        let mut transport = Self {
            context,
            card: None,
            reader_name,
            config,
            transaction_active: false,
        };

        // Try to connect to the card
        let _ = transport.connect_card();

        Ok(transport)
    }

    /// Try to connect to the card
    fn connect_card(&mut self) -> Result<(), PcscError> {
        if self.card.is_some() {
            return Ok(());
        }

        // Try to connect
        let reader_cstr = match CString::new(self.reader_name.clone()) {
            Ok(cstr) => cstr,
            Err(_) => return Err(PcscError::ReaderNotFound(self.reader_name.clone())),
        };

        match self.context.connect(
            &reader_cstr,
            self.config.share_mode.into(),
            self.config.protocols,
        ) {
            Ok(card) => {
                self.card = Some(card);
                Ok(())
            }
            Err(pcsc::Error::NoSmartcard) => Err(PcscError::NoCard(self.reader_name.clone())),
            Err(e) => Err(e.into()),
        }
    }

    /// Get the ATR of the current card
    pub fn atr(&self) -> Result<Vec<u8>, PcscError> {
        self.card.as_ref().map_or_else(
            || Err(PcscError::NoCard(self.reader_name.clone())),
            |card| {
                card.get_attribute_owned(pcsc::Attribute::AtrString)
                    .map_err(|e| e.into())
            },
        )
    }

    /// Get the reader name
    pub const fn reader_name(&self) -> &String {
        &self.reader_name
    }

    /// Check if the transport is connected to a card
    pub const fn has_card(&self) -> bool {
        self.card.is_some()
    }

    /// Transmit a command to the card
    fn transmit_command(&mut self, command: &[u8]) -> Result<Bytes, PcscError> {
        // Connect if needed
        self.connect_card()?;

        // Get a reference to the card
        let card = match &mut self.card {
            Some(card) => card,
            None => return Err(PcscError::NoCard(self.reader_name.clone())),
        };

        // Allocate a buffer for the response
        let mut response_buffer = [0u8; 258];

        // Send the command
        match card.transmit(command, &mut response_buffer) {
            Ok(response) => Ok(Bytes::copy_from_slice(response)),
            Err(e) => {
                // If card was reset or removed, clear our reference
                if matches!(e, pcsc::Error::ResetCard | pcsc::Error::RemovedCard) {
                    self.card = None;
                    self.transaction_active = false;

                    // Try to reconnect if needed
                    if self.config.auto_reconnect && e == pcsc::Error::ResetCard {
                        if let Ok(()) = self.connect_card() {
                            // Try again with the new connection
                            return self.transmit_command(command);
                        }
                    }
                }

                Err(e.into())
            }
        }
    }
}

impl CardTransport for PcscTransport {
    fn do_transmit_raw(&mut self, command: &[u8]) -> Result<Bytes, TransportError> {
        // Direct transmission without transaction handling
        self.transmit_command(command).map_err(TransportError::from)
    }

    fn is_connected(&self) -> bool {
        self.card.is_some()
    }

    fn reset(&mut self) -> Result<(), TransportError> {
        // End any active transaction
        self.transaction_active = false;

        // Disconnect from the card
        if let Some(card) = self.card.take() {
            let _ = card.disconnect(Disposition::ResetCard);
        }

        // Try to reconnect
        self.connect_card().map_err(Into::into)
    }
}

impl Drop for PcscTransport {
    fn drop(&mut self) {
        // End any active transaction
        self.transaction_active = false;

        // Disconnect from the card
        if let Some(card) = self.card.take() {
            let _ = card.disconnect(Disposition::LeaveCard);
        }
    }
}
