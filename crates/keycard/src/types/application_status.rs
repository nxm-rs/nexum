use std::fmt;

use iso7816_tlv::ber::{Tag, Tlv, Value};

use crate::tags;

use super::get_primitive_value;

/// Application status returned by GET STATUS P1=0x00 command
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApplicationStatus {
    /// PIN retry count
    pub pin_retry_count: u8,
    /// PUK retry count
    pub puk_retry_count: u8,
    /// Key initialized flag
    pub key_initialized: bool,
}

impl TryFrom<Tlv> for ApplicationStatus {
    type Error = crate::Error;

    fn try_from(tlv: Tlv) -> Result<Self, Self::Error> {
        if tlv.tag() != &Tag::try_from(tags::TEMPLATE_APPLICATION_STATUS)? {
            return Err(Self::Error::InvalidData(
                "TLV tag was not application status template tag",
            ));
        }

        match tlv.value() {
            Value::Primitive(_) => Err(Self::Error::InvalidData("TLV value was not constructed")),
            Value::Constructed(tlvs) => Ok(Self {
                pin_retry_count: get_primitive_value(&Tag::try_from(tags::OTHER)?, &tlvs[0])?[0],
                puk_retry_count: get_primitive_value(&Tag::try_from(tags::OTHER)?, &tlvs[1])?[0],
                key_initialized: get_primitive_value(
                    &Tag::try_from(tags::KEY_INITIALIZED)?,
                    &tlvs[2],
                )?[0]
                    == 0xFF,
            }),
        }
    }
}

impl TryFrom<&[u8]> for ApplicationStatus {
    type Error = crate::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tlv, _) = Tlv::parse(bytes);
        Self::try_from(tlv?)
    }
}

impl fmt::Display for ApplicationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Application Status:")?;
        writeln!(f, "  PIN retries remaining: {}", self.pin_retry_count)?;
        writeln!(f, "  PUK retries remaining: {}", self.puk_retry_count)?;
        write!(f, "  Key initialized: {}", self.key_initialized)
    }
}
