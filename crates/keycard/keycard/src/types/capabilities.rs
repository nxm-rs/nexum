use std::fmt;

use iso7816_tlv::ber::{Tlv, Value};

/// Capability flags for the keycard
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Capability {
    SecureChannel = 0x01,
    KeyManagement = 0x02,
    CredentialsManagement = 0x04,
    Ndef = 0x08,
}

/// Capabilities flags container
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capabilities(u8);

impl fmt::Display for Capabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut capabilities = Vec::new();
        if self.has_capability(Capability::SecureChannel) {
            capabilities.push("Secure Channel");
        }
        if self.has_capability(Capability::KeyManagement) {
            capabilities.push("Key Management");
        }
        if self.has_capability(Capability::CredentialsManagement) {
            capabilities.push("Credentials Management");
        }
        if self.has_capability(Capability::Ndef) {
            capabilities.push("NDEF");
        }
        write!(f, "{}", capabilities.join(", "))
    }
}

impl Capabilities {
    pub fn new(capabilities: &[Capability]) -> Self {
        Self(capabilities.iter().fold(0, |flags, &cap| flags | cap as u8))
    }

    /// Create a new empty capabilities set with no capabilities
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.0 & capability as u8 != 0
    }

    /// Check if a capability is supported, returning an error if not
    pub fn require_capability(&self, capability: Capability) -> crate::Result<()> {
        if !self.has_capability(capability) {
            let error_message = match capability {
                Capability::SecureChannel => {
                    "This card does not support the secure channel protocol"
                }
                Capability::KeyManagement => "This card does not support key management operations",
                Capability::CredentialsManagement => {
                    "This card does not support credentials management operations"
                }
                Capability::Ndef => "This card does not support NDEF operations",
            };
            Err(crate::Error::CapabilityNotSupported(error_message))
        } else {
            Ok(())
        }
    }
}

impl TryFrom<&Tlv> for Capabilities {
    type Error = crate::Error;

    fn try_from(tlv: &Tlv) -> Result<Self, Self::Error> {
        match tlv.value() {
            Value::Primitive(data) => Ok(data[0].into()),
            _ => Err(Self::Error::InvalidData("Invalid TLV for Capabilities")),
        }
    }
}

impl From<u8> for Capabilities {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
