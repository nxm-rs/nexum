use coins_bip32::path::DerivationPath;

pub mod derive_key;
pub use derive_key::*;
pub mod export_key;
pub use export_key::*;
pub mod factory_reset;
pub use factory_reset::*;
pub mod generate_key;
pub use generate_key::*;
pub mod generate_mnemonic;
pub use generate_mnemonic::*;
pub mod get_data;
pub use get_data::*;
pub mod get_status;
pub use get_status::*;
pub mod ident;
pub use ident::*;
pub mod init;
pub use init::*;
pub mod load_key;
pub use load_key::*;
pub mod mutually_authenticate;
pub use mutually_authenticate::*;
pub mod open_secure_channel;
pub use open_secure_channel::*;
pub mod pair;
pub use pair::*;
pub mod pin;
pub use pin::*;
pub mod remove_key;
pub use remove_key::*;
pub mod select;
pub use select::*;
pub mod set_pinless_path;
pub use set_pinless_path::*;
pub mod sign;
pub use sign::*;
pub mod store_data;
pub use store_data::*;
pub mod unpair;
pub use unpair::*;

use crate::Error;

pub const CLA_GP: u8 = 0x80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistentRecord {
    /// Store general public data
    Public = 0x00,
    /// Store data in the NDEF record
    Ndef = 0x01,
    /// Store data in the cashcard record
    Cashcard = 0x02,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPath {
    /// Use the current key path (no derivation)
    Current,
    /// Derive from master key
    FromMaster(Option<DerivationPath>),
    /// Derive from parent key
    FromParent(DerivationPath),
    /// Derive from current key
    FromCurrent(DerivationPath),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeriveMode {
    /// Derive without changing the current path (0x01)
    Temporary = 0x01,
    /// Derive and make it the new current path (0x02)
    Persistent = 0x02,
}

/// Helper function to convert a derivation path to bytes
fn path_to_bytes(path: &DerivationPath) -> Vec<u8> {
    path.iter()
        .flat_map(|&component| component.to_be_bytes())
        .collect()
}

/// Prepares parameters for key derivation and export commands
pub(crate) fn prepare_derivation_parameters(
    key_path: &KeyPath,
    derive_mode: Option<DeriveMode>,
) -> Result<(u8, Option<Vec<u8>>), Error> {
    match key_path {
        KeyPath::Current => {
            // No derivation, using current key
            if derive_mode.is_some() {
                return Err(crate::Error::InvalidDerivationArguments(
                    "Derive mode should not be specified when using current key".into(),
                ));
            }
            Ok((0x00, None))
        }
        KeyPath::FromMaster(path_opt) => {
            // Derive from master
            let derive_option = match derive_mode {
                None => {
                    return Err(Error::InvalidDerivationArguments(
                        "Derive mode must be specified when deriving".into(),
                    ));
                }
                Some(mode) => mode as u8,
            };

            let source_option = 0x00; // Master source
            let p1 = derive_option | source_option;

            // Convert path to bytes if provided, otherwise None
            let data = path_opt.as_ref().map(path_to_bytes);

            Ok((p1, data))
        }
        KeyPath::FromParent(path) | KeyPath::FromCurrent(path) => {
            // Derive from parent or current
            let derive_option = match derive_mode {
                None => {
                    return Err(Error::InvalidDerivationArguments(
                        "Derive mode must be specified when deriving".into(),
                    ));
                }
                Some(mode) => mode as u8,
            };

            // Set the source option based on key path
            let source_option = match key_path {
                KeyPath::FromParent(_) => 0x40,  // Parent source
                KeyPath::FromCurrent(_) => 0x80, // Current source
                _ => unreachable!(), // We're in a match arm that only handles these two variants
            };

            let p1 = derive_option | source_option;

            // Convert path to bytes
            let data = Some(path_to_bytes(path));

            Ok((p1, data))
        }
    }
}
