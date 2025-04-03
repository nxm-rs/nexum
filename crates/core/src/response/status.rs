//! Status word definitions for APDU responses

use std::fmt;

use tracing::Level;

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

    /// Check if this status word indicates normal processing (61 XX)
    pub const fn is_normal_processing(&self) -> bool {
        self.sw1 == 0x61
    }

    /// Check if this status word indicates more data is available (61 XX)
    pub const fn is_more_data_available(&self) -> bool {
        self.sw1 == 0x61
    }

    /// Get the number of remaining bytes when SW1 = 61
    pub const fn remaining_bytes(&self) -> Option<u8> {
        if self.sw1 == 0x61 {
            Some(self.sw2)
        } else {
            None
        }
    }

    /// Check if this status word indicates a warning (62 XX)
    pub const fn is_warning(&self) -> bool {
        self.sw1 == 0x62
    }

    /// Check if this status word indicates a security condition not satisfied (69 82)
    pub const fn is_security_condition_not_satisfied(&self) -> bool {
        self.sw1 == 0x69 && self.sw2 == 0x82
    }

    /// Check if this status word indicates a file not found (6A 82)
    pub const fn is_file_not_found(&self) -> bool {
        self.sw1 == 0x6A && self.sw2 == 0x82
    }

    /// Check if this status word indicates an incorrect P1 or P2 parameter (6A 86)
    pub const fn is_incorrect_p1p2(&self) -> bool {
        self.sw1 == 0x6A && self.sw2 == 0x86
    }

    /// Check if this status word indicates incorrect parameters (6A XX)
    pub const fn is_incorrect_parameters(&self) -> bool {
        self.sw1 == 0x6A
    }

    /// Check if this status word indicates wrong length (67 00)
    pub const fn is_wrong_length(&self) -> bool {
        self.sw1 == 0x67 && self.sw2 == 0x00
    }

    /// Check if this status word indicates a command not allowed (69 86)
    pub const fn is_command_not_allowed(&self) -> bool {
        self.sw1 == 0x69 && self.sw2 == 0x86
    }

    /// Get the appropriate tracing level for this status word
    pub const fn tracing_level(&self) -> Level {
        if self.is_success() || self.is_normal_processing() {
            Level::DEBUG
        } else if self.sw1 == 0x62 || self.sw1 == 0x63 {
            // Warnings
            Level::INFO
        } else {
            // Errors
            Level::WARN
        }
    }

    /// Get a description of this status word
    pub const fn description(&self) -> &'static str {
        match (self.sw1, self.sw2) {
            (0x90, 0x00) => "Success",
            (0x61, _) => "More data available",
            (0x62, 0x00) => "No information given",
            (0x62, 0x81) => "Part of returned data may be corrupted",
            (0x62, 0x82) => "End of file/record reached before reading Le bytes",
            (0x62, 0x83) => "Selected file invalidated",
            (0x62, 0x84) => "FCI not formatted according to specification",
            (0x63, 0x00) => "No information given",
            (0x63, 0x81) => "File filled up by the last write",
            (0x63, n) if (n & 0xF0) == 0xC0 => "Counter value",
            (0x64, 0x00) => "State of non-volatile memory unchanged",
            (0x65, 0x00) => "State of non-volatile memory changed",
            (0x65, 0x81) => "Memory failure",
            (0x67, 0x00) => "Wrong length",
            (0x68, 0x81) => "Logical channel not supported",
            (0x68, 0x82) => "Secure messaging not supported",
            (0x69, 0x81) => "Command incompatible with file structure",
            (0x69, 0x82) => "Security status not satisfied",
            (0x69, 0x83) => "Authentication method blocked",
            (0x69, 0x84) => "Referenced data invalidated",
            (0x69, 0x85) => "Conditions of use not satisfied",
            (0x69, 0x86) => "Command not allowed",
            (0x69, 0x87) => "Expected SM data objects missing",
            (0x69, 0x88) => "SM data objects incorrect",
            (0x6A, 0x80) => "Incorrect parameters in the data field",
            (0x6A, 0x81) => "Function not supported",
            (0x6A, 0x82) => "File not found",
            (0x6A, 0x83) => "Record not found",
            (0x6A, 0x84) => "Not enough memory space in the file",
            (0x6A, 0x85) => "Lc inconsistent with TLV structure",
            (0x6A, 0x86) => "Incorrect parameters P1-P2",
            (0x6A, 0x87) => "Lc inconsistent with P1-P2",
            (0x6A, 0x88) => "Referenced data not found",
            (0x6B, 0x00) => "Wrong parameters P1-P2",
            (0x6C, _) => "Wrong Le field",
            (0x6D, 0x00) => "Instruction code not supported or invalid",
            (0x6E, 0x00) => "Class not supported",
            (0x6F, 0x00) => "No precise diagnosis",
            _ => "Unknown status word",
        }
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

/// Common status words
pub mod common {
    use super::StatusWord;

    /// Success (90 00)
    pub const SUCCESS: StatusWord = StatusWord::new(0x90, 0x00);

    /// More data available (61 XX) - XX is the number of remaining bytes
    pub const MORE_DATA: StatusWord = StatusWord::new(0x61, 0x00);

    /// Warning, non-volatile memory unchanged (62 00)
    pub const WARNING: StatusWord = StatusWord::new(0x62, 0x00);

    /// Wrong length (67 00)
    pub const WRONG_LENGTH: StatusWord = StatusWord::new(0x67, 0x00);

    /// Command not allowed (69 86)
    pub const COMMAND_NOT_ALLOWED: StatusWord = StatusWord::new(0x69, 0x86);

    /// Security condition not satisfied (69 82)
    pub const SECURITY_CONDITION_NOT_SATISFIED: StatusWord = StatusWord::new(0x69, 0x82);

    /// Function not supported (6A 81)
    pub const FUNCTION_NOT_SUPPORTED: StatusWord = StatusWord::new(0x6A, 0x81);

    /// File not found (6A 82)
    pub const FILE_NOT_FOUND: StatusWord = StatusWord::new(0x6A, 0x82);

    /// Record not found (6A 83)
    pub const RECORD_NOT_FOUND: StatusWord = StatusWord::new(0x6A, 0x83);

    /// Incorrect parameters P1-P2 (6A 86)
    pub const INCORRECT_P1P2: StatusWord = StatusWord::new(0x6A, 0x86);

    /// Incorrect parameter (data field) (6A 80)
    pub const INCORRECT_DATA: StatusWord = StatusWord::new(0x6A, 0x80);

    /// Command incompatible with file structure (6981)
    pub const COMMAND_INCOMPATIBLE: StatusWord = StatusWord::new(0x69, 0x81);

    /// Invalid instruction (6D 00)
    pub const INVALID_INSTRUCTION: StatusWord = StatusWord::new(0x6D, 0x00);

    /// Class not supported (6E 00)
    pub const CLASS_NOT_SUPPORTED: StatusWord = StatusWord::new(0x6E, 0x00);
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
        assert!(StatusWord::new(0x61, 0x10).is_more_data_available());
        assert!(StatusWord::new(0x62, 0x83).is_warning());
        assert!(StatusWord::new(0x67, 0x00).is_wrong_length());
        assert!(StatusWord::new(0x69, 0x82).is_security_condition_not_satisfied());
        assert!(StatusWord::new(0x6A, 0x82).is_file_not_found());
        assert!(StatusWord::new(0x6A, 0x86).is_incorrect_p1p2());
        assert!(StatusWord::new(0x6A, 0x88).is_incorrect_parameters());
    }

    #[test]
    fn test_status_word_remaining_bytes() {
        assert_eq!(StatusWord::new(0x61, 0x15).remaining_bytes(), Some(0x15));
        assert_eq!(StatusWord::new(0x90, 0x00).remaining_bytes(), None);
    }

    #[test]
    fn test_status_word_description() {
        assert_eq!(StatusWord::new(0x90, 0x00).description(), "Success");
        assert_eq!(
            StatusWord::new(0x61, 0x15).description(),
            "More data available"
        );
        assert_eq!(StatusWord::new(0x6A, 0x82).description(), "File not found");
        assert_eq!(
            StatusWord::new(0x69, 0x82).description(),
            "Security status not satisfied"
        );
    }
}
