use std::fmt;

use iso7816_tlv::ber::{Tag, Tlv};
use nexum_apdu_globalplatform::commands::select::SelectOk;

use crate::constants::tags;
use crate::types::ApplicationInfo;

impl TryFrom<SelectOk> for ParsedSelectOk {
    type Error = crate::Error;

    fn try_from(response: SelectOk) -> Result<Self, Self::Error> {
        match response {
            SelectOk::Success { fci } => ParsedSelectOk::try_from(fci.as_slice()),
        }
    }
}

#[derive(Debug)]
pub enum ParsedSelectOk {
    /// Regular response with application info for an initialized card
    InitializedWithKey(ApplicationInfo),
    /// Response for a card that's not yet initialized (no master key generated/loaded)
    InitializedNoKey(ApplicationInfo),
    /// Response in pre-initialized state (only public key)
    Uninitialized(Option<k256::PublicKey>),
}

impl fmt::Display for ParsedSelectOk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParsedSelectOk::InitializedWithKey(info) => write!(f, "{}", info),
            ParsedSelectOk::InitializedNoKey(info) => {
                writeln!(f, "Initialized Card (with no key):")?;
                write!(f, "{}", info)
            }
            ParsedSelectOk::Uninitialized(maybe_key) => {
                writeln!(f, "Un-initialized State:")?;
                match &maybe_key {
                    Some(key) => write!(f, "  Public Key: {:#?}", key),
                    None => write!(f, "  Public Key: None"),
                }
            }
        }
    }
}

impl TryFrom<&[u8]> for ParsedSelectOk {
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let fci = Tlv::from_bytes(value)?;

        let application_info = Tag::try_from(tags::TEMPLATE_APPLICATION_INFO)?;
        let ecc_public_key = Tag::try_from(tags::ECC_PUBLIC_KEY)?;

        if fci.tag() == &application_info {
            let app_info = ApplicationInfo::try_from(&fci)?;

            // Determine if the card is initialized by checking for key_uid
            if app_info.key_uid.is_some() {
                Ok(ParsedSelectOk::InitializedWithKey(app_info))
            } else {
                Ok(ParsedSelectOk::InitializedNoKey(app_info))
            }
        } else if fci.tag() == &ecc_public_key {
            Ok(ParsedSelectOk::Uninitialized(
                *crate::types::PublicKey::try_from(&fci)?,
            ))
        } else {
            Err(Self::Error::InvalidData("Invalid Tag"))
        }
    }
}
