//! Status word definitions for APDU responses

use std::fmt;

/// Status Word (SW1-SW2) from an APDU response
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusWord {
    /// First status byte (SW1)
    pub sw1: u8,
    /// Second status byte (SW2)
    pub sw2: u8,
}

impl StatusWord {
    /// Create a new status word
    pub const fn new(sw1: u8, sw2: u8) -> Self {
        Self { sw1, sw2 }
    }

    /// Create from a u16 value (SW1 | SW2)
    pub const fn from_u16(status: u16) -> Self {
        Self {
            sw1: (status >> 8) as u8,
            sw2: status as u8,
        }
    }

    /// Convert to a u16 value (SW1 | SW2)
    pub const fn to_u16(&self) -> u16 {
        ((self.sw1 as u16) << 8) | (self.sw2 as u16)
    }

    /// Check if this status word indicates success (90 00)
    pub const fn is_success(&self) -> bool {
        self.sw1 == 0x90 && self.sw2 == 0x00
    }
}

impl From<(u8, u8)> for StatusWord {
    fn from(tuple: (u8, u8)) -> Self {
        Self::new(tuple.0, tuple.1)
    }
}

impl From<u16> for StatusWord {
    fn from(status: u16) -> Self {
        Self::from_u16(status)
    }
}

impl From<StatusWord> for u16 {
    fn from(status: StatusWord) -> Self {
        status.to_u16()
    }
}

impl fmt::Display for StatusWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X} {:02X}", self.sw1, self.sw2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_word_from_to_u16() {
        let sw = StatusWord::from_u16(0x9000);
        assert_eq!(sw.sw1, 0x90);
        assert_eq!(sw.sw2, 0x00);
        assert_eq!(sw.to_u16(), 0x9000);
    }

    #[test]
    fn test_status_word_is_methods() {
        assert!(StatusWord::new(0x90, 0x00).is_success());
    }
}
