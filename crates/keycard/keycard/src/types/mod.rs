mod application_info;
mod application_status;
mod capabilities;
mod exported_key;
mod keypair;
mod pairing_info;
mod signature;
mod version;

pub use application_info::ApplicationInfo;
pub use application_status::ApplicationStatus;
pub use capabilities::{Capabilities, Capability};
pub use exported_key::ExportedKey;
use iso7816_tlv::ber::{Tag, Tlv, Value};
pub use keypair::*;
pub use pairing_info::PairingInfo;
pub use signature::*;
pub use version::Version;

use crate::Error;

pub(crate) fn get_primitive_value(tag: &Tag, tlv: &Tlv) -> Result<Vec<u8>, Error> {
    if tag != tlv.tag() {
        return Err(Error::InvalidData("Invalid tag"));
    }
    match tlv.value() {
        Value::Primitive(bytes) => Ok(bytes.to_vec()),
        _ => Err(Error::InvalidData("Invalid value type")),
    }
}
