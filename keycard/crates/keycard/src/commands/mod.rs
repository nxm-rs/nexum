pub mod derive_key;
use bytes::{Bytes, BytesMut};
use coins_bip32::path::DerivationPath;
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

pub const CLA_GP: u8 = 0x80;

pub const DERIVE_FROM_MASTER: u8 = 0x01;
pub const DERIVE_FROM_PINLESS: u8 = 0x03;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum PersistentRecord {
    /// Store general public data
    Public = 0x00,
    /// Store data in the NDEF record
    Ndef = 0x01,
    /// Store data in the cashcard record
    Cashcard = 0x02,
}

pub(crate) fn derivation_path_to_bytes(path: &DerivationPath) -> Bytes {
    path.iter()
        .fold(BytesMut::new(), |mut bytes, component| {
            bytes.extend_from_slice(&component.to_be_bytes());
            bytes
        })
        .freeze()
}
