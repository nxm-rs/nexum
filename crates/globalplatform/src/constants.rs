//! Constants used in GlobalPlatform operations
//!
//! This module contains various constants defined by the GlobalPlatform specification,
//! such as CLA bytes, instruction codes, parameter values, and status words.

/// GlobalPlatform command classes
pub mod cla {
    /// ISO7816 command class
    pub const ISO7816: u8 = 0x00;
    /// GlobalPlatform command class
    pub const GP: u8 = 0x80;
    /// Secure messaging command class (with MAC)
    pub const MAC: u8 = 0x84;
}

/// GlobalPlatform instruction codes
pub mod ins {
    /// SELECT command
    pub const SELECT: u8 = 0xA4;
    /// INITIALIZE UPDATE command
    pub const INITIALIZE_UPDATE: u8 = 0x50;
    /// EXTERNAL AUTHENTICATE command
    pub const EXTERNAL_AUTHENTICATE: u8 = 0x82;
    /// GET RESPONSE command
    pub const GET_RESPONSE: u8 = 0xC0;
    /// DELETE command
    pub const DELETE: u8 = 0xE4;
    /// LOAD command
    pub const LOAD: u8 = 0xE8;
    /// INSTALL command
    pub const INSTALL: u8 = 0xE6;
    /// GET STATUS command
    pub const GET_STATUS: u8 = 0xF2;
    /// PUT KEY command
    pub const PUT_KEY: u8 = 0xD8;
    /// STORE DATA command
    pub const STORE_DATA: u8 = 0xE2;
}

/// Parameter values for SELECT command (P1)
pub mod select_p1 {
    /// Select by DF name
    pub const BY_NAME: u8 = 0x04;
}

/// Parameter values for EXTERNAL AUTHENTICATE command (P1)
pub mod external_auth_p1 {
    /// Authenticate using CMAC
    pub const CMAC: u8 = 0x01;
    /// Authenticate using RMAC
    pub const RMAC: u8 = 0x10;
    /// Authenticate using ENC
    pub const ENC: u8 = 0x20;
    /// Authenticate using RENC
    pub const RENC: u8 = 0x40;
}

/// Parameter values for INSTALL command (P1)
pub mod install_p1 {
    /// Install for load
    pub const FOR_LOAD: u8 = 0x02;
    /// Install for install
    pub const FOR_INSTALL: u8 = 0x04;
    /// Install for make selectable
    pub const FOR_MAKE_SELECTABLE: u8 = 0x08;
    /// Install for install and make selectable
    pub const FOR_INSTALL_AND_MAKE_SELECTABLE: u8 = FOR_INSTALL | FOR_MAKE_SELECTABLE;
    /// Install for extradition
    pub const FOR_EXTRADITION: u8 = 0x10;
    /// Install for personalization
    pub const FOR_PERSONALIZATION: u8 = 0x20;
    /// Install for registry update
    pub const FOR_REGISTRY_UPDATE: u8 = 0x40;
}

/// Parameter values for LOAD command (P1)
pub mod load_p1 {
    /// More blocks to follow
    pub const MORE_BLOCKS: u8 = 0x00;
    /// Last block
    pub const LAST_BLOCK: u8 = 0x80;
}

/// Parameter values for GET STATUS command (P1)
pub mod get_status_p1 {
    /// Get status of issuer security domain
    pub const ISSUER_SECURITY_DOMAIN: u8 = 0x80;
    /// Get status of applications
    pub const APPLICATIONS: u8 = 0x40;
    /// Get status of executable load files
    pub const EXEC_LOAD_FILES: u8 = 0x20;
    /// Get status of executable load files and modules
    pub const EXEC_LOAD_FILES_AND_MODULES: u8 = 0x10;
}

/// Parameter values for GET STATUS command (P2)
pub mod get_status_p2 {
    /// Return data in TLV format
    pub const TLV_DATA: u8 = 0x02;
}

/// Parameter values for DELETE command (P2)
pub mod delete_p2 {
    /// Delete object
    pub const OBJECT: u8 = 0x00;
    /// Delete object and related objects
    pub const OBJECT_AND_RELATED: u8 = 0x80;
}

/// Commonly used status words in GlobalPlatform
pub mod status {
    use nexum_apdu_core::StatusWord;

    pub use gp::*;
    pub use iso7816::*;

    pub mod iso7816 {
        use super::StatusWord;

        /// Success (No Error)
        pub const SW_NO_ERROR: StatusWord = StatusWord::new(0x90, 0x00);
        /// Response data incomplete (SW1 = 0x61)
        pub const SW_BYTES_REMAINING_00: StatusWord = StatusWord::new(0x61, 0x00);
        /// Wrong length
        pub const SW_WRONG_LENGTH: StatusWord = StatusWord::new(0x67, 0x00);
        /// Wrong data
        pub const SW_WRONG_DATA: StatusWord = StatusWord::new(0x6A, 0x80);
        /// File not found
        pub const SW_FILE_NOT_FOUND: StatusWord = StatusWord::new(0x6A, 0x82);
        /// Referenced data not found (changed to RECORD_NOT_FOUND for compatibility)
        pub const SW_RECORD_NOT_FOUND: StatusWord = StatusWord::new(0x6A, 0x83);
        /// Security condition not satisfied
        pub const SW_SECURITY_STATUS_NOT_SATISFIED: StatusWord = StatusWord::new(0x69, 0x82);
        /// Authentication method blocked (changed to FILE_INVALID for compatibility)
        pub const SW_FILE_INVALID: StatusWord = StatusWord::new(0x69, 0x83);
        /// Command not allowed
        pub const SW_COMMAND_NOT_ALLOWED: StatusWord = StatusWord::new(0x69, 0x86);
        /// Incorrect parameters (P1,P2)
        pub const SW_INCORRECT_P1P2: StatusWord = StatusWord::new(0x6A, 0x86);
        /// Wrong parameters P1-P2
        pub const SW_WRONG_P1P2: StatusWord = StatusWord::new(0x6B, 0x00);
        /// Function not supported
        pub const SW_FUNC_NOT_SUPPORTED: StatusWord = StatusWord::new(0x6A, 0x81);
        /// Conditions of use not satisfied
        pub const SW_CONDITIONS_NOT_SATISFIED: StatusWord = StatusWord::new(0x69, 0x85);
        /// CLA value not supported
        pub const SW_CLA_NOT_SUPPORTED: StatusWord = StatusWord::new(0x6E, 0x00);
        /// INS value not supported
        pub const SW_INS_NOT_SUPPORTED: StatusWord = StatusWord::new(0x6D, 0x00);
        /// No precise diagnosis
        pub const SW_UNKNOWN: StatusWord = StatusWord::new(0x6F, 0x00);
        /// Warning, card state unchanged
        pub const SW_WARNING_STATE_UNCHANGED: StatusWord = StatusWord::new(0x62, 0x00);
        /// Data invalid
        pub const SW_DATA_INVALID: StatusWord = StatusWord::new(0x69, 0x84);
        /// Not enough memory space in the file
        pub const SW_FILE_FULL: StatusWord = StatusWord::new(0x6A, 0x84);
        /// Applet selection failed
        pub const SW_APPLET_SELECT_FAILED: StatusWord = StatusWord::new(0x69, 0x99);
    }

    pub mod gp {
        use super::StatusWord;

        /// Referenced data not found
        pub const SW_REFERENCED_DATA_NOT_FOUND: StatusWord = StatusWord::new(0x6A, 0x88);
    }
}

/// Tags used in GlobalPlatform commands and responses
pub mod tags {
    /// AID tag for DELETE command
    pub const DELETE_AID: u8 = 0x4F;
    /// Load file data block tag
    pub const LOAD_FILE_DATA_BLOCK: u8 = 0xC4;
    /// AID tag for GET STATUS command
    pub const GET_STATUS_AID: u8 = 0x4F;
    /// Application label tag
    pub const APPLICATION_LABEL: u8 = 0x50;
    /// Security domain management data
    pub const SD_MANAGEMENT_DATA: u8 = 0x73;
    /// Key diversification data
    pub const KEY_DIVERSIFICATION_DATA: u8 = 0xCF;
}

/// Secure Channel Protocol (SCP) versions
pub mod scp {
    /// SCP01 protocol version
    pub const SCP01: u8 = 0x01;
    /// SCP02 protocol version
    pub const SCP02: u8 = 0x02;
    /// SCP03 protocol version
    pub const SCP03: u8 = 0x03;
}

/// Default host challenge length in bytes
pub const DEFAULT_HOST_CHALLENGE_LENGTH: usize = 8;

/// Default card challenge length in bytes
pub const DEFAULT_CARD_CHALLENGE_LENGTH: usize = 8;

/// Security domain AID (ISD)
pub const SECURITY_DOMAIN_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x01, 0x51, 0x00, 0x00, 0x00];
