//! APDU command definitions and traits
//!
//! This module provides types and traits for working with APDU commands
//! according to ISO/IEC 7816-4.

use std::fmt;

use bytes::{BufMut, Bytes, BytesMut};

#[cfg(feature = "longer_payloads")]
/// Expected length type for APDU commands
pub type ExpectedLength = u16;
#[cfg(not(feature = "longer_payloads"))]
/// Expected length type for APDU commands
pub type ExpectedLength = u8;

use crate::{Error, Response, prelude::SecurityLevel};

/// Core trait for APDU commands
pub trait ApduCommand {
    /// Success response type
    type Success;

    /// Error response type
    type Error: fmt::Debug;
    
    /// Convert core Error to command-specific error type
    fn convert_error(error: Error) -> Self::Error;

    /// Command class (CLA)
    fn class(&self) -> u8;

    /// Instruction code (INS)
    fn instruction(&self) -> u8;

    /// First parameter (P1)
    fn p1(&self) -> u8;

    /// Second parameter (P2)
    fn p2(&self) -> u8;

    /// Command payload data (optional)
    fn data(&self) -> Option<&[u8]>;

    /// Expected response length (optional)
    fn expected_length(&self) -> Option<ExpectedLength>;

    /// Convert to raw APDU bytes
    fn to_bytes(&self) -> Bytes {
        let mut buffer = BytesMut::with_capacity(self.command_length());

        // Header: CLA, INS, P1, P2
        buffer.put_u8(self.class());
        buffer.put_u8(self.instruction());
        buffer.put_u8(self.p1());
        buffer.put_u8(self.p2());

        // Add Lc and data if present
        if let Some(data) = self.data() {
            let data_len = data.len();
            buffer.put_u8(data_len as u8);
            buffer.put_slice(data);
        }

        // Add Le if present
        if let Some(le) = self.expected_length() {
            #[cfg(feature = "longer_payloads")]
            {
                if le > 255 {
                    // For values > 255, use extended format
                    buffer.put_u8((le >> 8) as u8);
                    buffer.put_u8((le & 0xFF) as u8);
                } else {
                    buffer.put_u8(le as u8);
                }
            }

            #[cfg(not(feature = "longer_payloads"))]
            {
                buffer.put_u8(le);
            }
        }

        buffer.freeze()
    }

    /// Calculate length of serialized command
    fn command_length(&self) -> usize {
        // Header (CLA, INS, P1, P2) is always 4 bytes
        let mut length = 4;

        // Add Lc, data length if present
        if let Some(data) = self.data() {
            let data_len = data.len();

            #[cfg(feature = "longer_payloads")]
            if data_len > 255 {
                // Extended length: 00 + 2 bytes length
                length += 3 + data_len;
            } else {
                // Standard length: 1 byte
                length += 1 + data_len;
            }

            #[cfg(not(feature = "longer_payloads"))]
            {
                length += 1 + data_len;
            }
        }

        // Add Le if present
        if let Some(_le) = self.expected_length() {
            #[cfg(feature = "longer_payloads")]
            {
                if _le > 256 {
                    // For extended length, add 2 bytes or 3 bytes if no data
                    length += if self.data().is_some() { 2 } else { 3 };
                } else {
                    // Standard length: 1 byte
                    length += 1;
                }
            }

            #[cfg(not(feature = "longer_payloads"))]
            {
                length += 1;
            }
        }

        length
    }

    /// The security level that this command requires, defaulting to none
    fn required_security_level(&self) -> SecurityLevel {
        SecurityLevel::none()
    }

    /// Convert to a generic Command
    fn to_command(&self) -> Command {
        Command {
            cla: self.class(),
            ins: self.instruction(),
            p1: self.p1(),
            p2: self.p2(),
            data: self.data().map(Bytes::copy_from_slice),
            le: self.expected_length(),
        }
    }

    /// Parse response into the command's response type
    fn parse_response(response: Response) -> Result<Self::Success, Self::Error>;

    /// Parse raw bytes into the command's response type
    fn parse_response_raw(bytes: Bytes) -> Result<Self::Success, Self::Error> {
        let response = Response::from_bytes(&bytes)
            .map_err(Self::convert_error)?;
        Self::parse_response(response)
    }
}

/// Generic APDU command structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Command class byte
    pub cla: u8,
    /// Instruction byte
    pub ins: u8,
    /// Parameter 1
    pub p1: u8,
    /// Parameter 2
    pub p2: u8,
    /// Command data (optional)
    pub data: Option<Bytes>,
    /// Expected length (optional)
    pub le: Option<ExpectedLength>,
}

impl Command {
    /// Create a new command with just the header bytes
    pub const fn new(cla: u8, ins: u8, p1: u8, p2: u8) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data: None,
            le: None,
        }
    }

    /// Create a new command with expected response length (Le)
    pub const fn new_with_le(cla: u8, ins: u8, p1: u8, p2: u8, le: ExpectedLength) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data: None,
            le: Some(le),
        }
    }

    /// Create a new command with data payload
    pub fn new_with_data<T: Into<Bytes>>(cla: u8, ins: u8, p1: u8, p2: u8, data: T) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data: Some(data.into()),
            le: None,
        }
    }

    /// Create a new command with both data and expected length
    pub fn new_with_data_and_le<T: Into<Bytes>>(
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: T,
        le: ExpectedLength,
    ) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data: Some(data.into()),
            le: Some(le),
        }
    }

    /// Set the data field
    pub fn with_data<T: Into<Bytes>>(mut self, data: T) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Set the expected length field
    pub const fn with_le(mut self, le: ExpectedLength) -> Self {
        self.le = Some(le);
        self
    }

    /// Parse a command from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 4 {
            return Err(Error::InvalidCommandLength(data.len()));
        }

        let cla = data[0];
        let ins = data[1];
        let p1 = data[2];
        let p2 = data[3];

        let mut command = Self::new(cla, ins, p1, p2);

        // Parse Lc, data, and Le if present
        if data.len() > 4 {
            // Standard case
            let lc = data[4] as usize;

            if data.len() == 5 {
                // Only Le present, no data
                command.le = Some(data[4] as ExpectedLength);
            } else if data.len() >= 5 + lc {
                if lc > 0 {
                    command.data = Some(Bytes::copy_from_slice(&data[5..5 + lc]));
                }

                // Check for Le
                if data.len() > 5 + lc {
                    if data.len() == 5 + lc + 1 {
                        command.le = Some(data[5 + lc] as ExpectedLength);
                    } else {
                        return Err(Error::InvalidCommandLength(data.len()));
                    }
                }
            } else {
                return Err(Error::InvalidCommandLength(data.len()));
            }
        }

        Ok(command)
    }
}

// Update the ApduCommand implementation
impl ApduCommand for Command {
    type Success = Response;
    type Error = Error;
    
    fn convert_error(error: Error) -> Self::Error {
        error
    }

    fn class(&self) -> u8 {
        self.cla
    }

    fn instruction(&self) -> u8 {
        self.ins
    }

    fn p1(&self) -> u8 {
        self.p1
    }

    fn p2(&self) -> u8 {
        self.p2
    }

    fn data(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }

    fn expected_length(&self) -> Option<ExpectedLength> {
        self.le
    }

    fn parse_response(response: Response) -> Result<Self::Success, Self::Error> {
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_serialization() {
        let data = Bytes::from_static(&[0xA0, 0x00, 0x00, 0x01, 0x51, 0x00]);
        let cmd = Command::new_with_data_and_le(0x00, 0xA4, 0x04, 0x00, data, 0);
        let bytes = cmd.to_bytes();

        assert_eq!(bytes[0], 0x00); // CLA
        assert_eq!(bytes[1], 0xA4); // INS
        assert_eq!(bytes[2], 0x04); // P1
        assert_eq!(bytes[3], 0x00); // P2
        assert_eq!(bytes[4], 0x06); // Lc (data length)
        assert_eq!(bytes[5], 0xA0); // Data
        assert_eq!(bytes[6], 0x00);
        assert_eq!(bytes[7], 0x00);
        assert_eq!(bytes[8], 0x01);
        assert_eq!(bytes[9], 0x51);
        assert_eq!(bytes[10], 0x00);
        assert_eq!(bytes[11], 0x00); // Le
    }

    #[test]
    fn test_command_length() {
        let cmd1 = Command::new(0x00, 0xB0, 0x00, 0x00);
        assert_eq!(cmd1.command_length(), 4);

        let cmd2 = Command::new_with_le(0x00, 0xB0, 0x00, 0x00, 0xFF);
        assert_eq!(cmd2.command_length(), 5);

        let data = Bytes::from_static(&[0x01, 0x02, 0x03]);
        let cmd3 = Command::new_with_data(0x00, 0xD6, 0x00, 0x00, data.clone());
        assert_eq!(cmd3.command_length(), 8);

        let cmd4 = Command::new_with_data_and_le(0x00, 0xD6, 0x00, 0x00, data, 0xFF);
        assert_eq!(cmd4.command_length(), 9);
    }

    #[test]
    fn test_command_from_bytes() {
        // Test case 1: Simple command with no data or Le
        let data = &[0x00, 0xA4, 0x04, 0x00];
        let cmd = Command::from_bytes(data).unwrap();
        assert_eq!(cmd.cla, 0x00);
        assert_eq!(cmd.ins, 0xA4);
        assert_eq!(cmd.p1, 0x04);
        assert_eq!(cmd.p2, 0x00);
        assert!(cmd.data.is_none());
        assert!(cmd.le.is_none());

        // Test case 2: Command with data but no Le
        let data = &[0x00, 0xA4, 0x04, 0x00, 0x03, 0x01, 0x02, 0x03];
        let cmd = Command::from_bytes(data).unwrap();
        assert_eq!(cmd.cla, 0x00);
        assert_eq!(cmd.ins, 0xA4);
        assert_eq!(cmd.p1, 0x04);
        assert_eq!(cmd.p2, 0x00);
        assert_eq!(cmd.data.as_ref().unwrap(), &[0x01, 0x02, 0x03].as_ref());
        assert!(cmd.le.is_none());

        // Test case 3: Command with data and Le
        let data = &[0x00, 0xA4, 0x04, 0x00, 0x03, 0x01, 0x02, 0x03, 0xFF];
        let cmd = Command::from_bytes(data).unwrap();
        assert_eq!(cmd.cla, 0x00);
        assert_eq!(cmd.ins, 0xA4);
        assert_eq!(cmd.p1, 0x04);
        assert_eq!(cmd.p2, 0x00);
        assert_eq!(cmd.data.as_ref().unwrap(), &[0x01, 0x02, 0x03].as_ref());
        assert_eq!(cmd.le.unwrap(), 0xFF);

        // Test case 4: Command with no data but with Le
        let data = &[0x00, 0xB0, 0x00, 0x00, 0xFF];
        let cmd = Command::from_bytes(data).unwrap();
        assert_eq!(cmd.cla, 0x00);
        assert_eq!(cmd.ins, 0xB0);
        assert_eq!(cmd.p1, 0x00);
        assert_eq!(cmd.p2, 0x00);
        assert!(cmd.data.is_none());
        assert_eq!(cmd.le.unwrap(), 0xFF);

        // Test case 5: Command with Le=0 (should be 0)
        let data = &[0x00, 0xB0, 0x00, 0x00, 0x00];
        let cmd = Command::from_bytes(data).unwrap();
        assert_eq!(cmd.le.unwrap(), 0);
    }
}
