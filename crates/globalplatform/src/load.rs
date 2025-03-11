//! CAP file loading functionality
//!
//! This module provides functionality for loading CAP (Converted APplet) files
//! to smart cards.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use bytes::{BufMut, BytesMut};

use crate::{Error, Result, constants::tags};

/// Maximum block size for LOAD commands
pub const BLOCK_SIZE: usize = 247; // 255 - 8 bytes for MAC

/// Internal file names in CAP file
const INTERNAL_FILES: &[&str] = &[
    "Header",
    "Directory",
    "Import",
    "Applet",
    "Class",
    "Method",
    "StaticField",
    "Export",
    "ConstantPool",
    "RefLocation",
    "Descriptor",
];

/// Callback function type for load progress
pub type LoadingCallback = dyn FnMut(usize, usize) -> Result<()>;

/// A stream of LOAD commands for a CAP file
#[derive(Debug)]
pub struct LoadCommandStream {
    /// CAP file data
    data: Vec<u8>,
    /// Current position in data
    position: usize,
    /// Total blocks count
    blocks_count: usize,
    /// Current block index
    current_block: usize,
}

impl LoadCommandStream {
    /// Create a new LoadCommandStream from a CAP file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_file_handle(file)
    }

    /// Create a new LoadCommandStream from a File handle
    pub fn from_file_handle(file: File) -> Result<Self> {
        let mut zip = ZipArchive::new(file).map_err(|_| Error::CapFile("Invalid ZIP file"))?;

        // Load files from the CAP archive
        let mut files = std::collections::HashMap::new();

        for file_name in INTERNAL_FILES {
            // Try both .cap and .CAP extensions and without extension
            let mut content = None;

            for &ext in &["", ".cap", ".CAP"] {
                let full_name = format!("{}{}", file_name, ext);

                if let Ok(mut file) = zip.by_name(&full_name) {
                    let mut data = Vec::new();
                    file.read_to_end(&mut data)?;
                    content = Some(data);
                    break;
                }
            }

            if let Some(data) = content {
                files.insert(*file_name, data);
            }
        }

        // Encode the files into the load file data block format
        let data = Self::encode_files_data(&files)?;
        let blocks_count = (data.len() + BLOCK_SIZE - 1) / BLOCK_SIZE; // ceiling division

        Ok(Self {
            data,
            position: 0,
            blocks_count,
            current_block: 0,
        })
    }

    /// Encode the files into the load file data block format
    fn encode_files_data(files: &std::collections::HashMap<&str, Vec<u8>>) -> Result<Vec<u8>> {
        let mut buf = BytesMut::new();

        // Append all files in the correct order
        for file_name in INTERNAL_FILES {
            if let Some(data) = files.get(file_name) {
                buf.put_slice(data);
            }
        }

        let files_data = buf.freeze();
        let length_bytes = Self::encode_length(files_data.len());

        // Build the final data block with tag, length, and data
        let mut data = BytesMut::with_capacity(1 + length_bytes.len() + files_data.len());
        data.put_u8(tags::LOAD_FILE_DATA_BLOCK);
        data.put_slice(&length_bytes);
        data.put_slice(&files_data);

        Ok(data.freeze().to_vec())
    }

    /// Encode a length value into BER-TLV format
    fn encode_length(length: usize) -> Vec<u8> {
        if length < 0x80 {
            // Short form
            return vec![length as u8];
        } else if length < 0x100 {
            // Long form, 1 byte
            return vec![0x81, length as u8];
        } else if length < 0x10000 {
            // Long form, 2 bytes
            return vec![0x82, (length >> 8) as u8, (length & 0xFF) as u8];
        } else {
            // Long form, 3 bytes
            return vec![
                0x83,
                (length >> 16) as u8,
                ((length >> 8) & 0xFF) as u8,
                (length & 0xFF) as u8,
            ];
        }
    }

    /// Get the total number of blocks
    pub fn blocks_count(&self) -> usize {
        self.blocks_count
    }

    /// Get the current block index
    pub fn current_block(&self) -> usize {
        self.current_block
    }

    /// Check if there are more blocks
    pub fn has_next(&self) -> bool {
        self.position < self.data.len()
    }

    /// Get the next block
    pub fn next_block(&mut self) -> Option<(bool, u8, &[u8])> {
        if !self.has_next() {
            return None;
        }

        let remaining = self.data.len() - self.position;
        let block_size = std::cmp::min(remaining, BLOCK_SIZE);
        let is_last = remaining <= BLOCK_SIZE;

        let block_index = self.current_block as u8;
        let block_data = &self.data[self.position..self.position + block_size];

        self.position += block_size;
        self.current_block += 1;

        Some((is_last, block_index, block_data))
    }

    /// Extract information about a CAP file
    pub fn extract_info<P: AsRef<Path>>(path: P) -> Result<CapFileInfo> {
        let file = File::open(path)?;
        let mut zip = ZipArchive::new(file).map_err(|_| Error::CapFile("Invalid ZIP file"))?;

        let mut info = CapFileInfo {
            package_aid: None,
            applet_aids: Vec::new(),
            version: None,
            files: Vec::new(),
        };

        // List all files in the archive
        for i in 0..zip.len() {
            if let Ok(file) = zip.by_index(i) {
                info.files.push(file.name().to_string());
            }
        }

        // Try to read Header.cap to get AID
        if let Ok(mut header_file) = zip.by_name("Header.cap") {
            let mut header_data = Vec::new();
            header_file.read_to_end(&mut header_data)?;

            if header_data.len() > 15 {
                // Extract package version
                if header_data.len() > 5 {
                    info.version = Some((header_data[4], header_data[5]));
                }

                // Extract package AID
                if header_data.len() > 13 && header_data[13] < 16 {
                    let aid_len = header_data[13] as usize;
                    if 14 + aid_len <= header_data.len() {
                        let mut package_aid = Vec::new();
                        package_aid.extend_from_slice(&header_data[14..14 + aid_len]);
                        info.package_aid = Some(package_aid);
                    }
                }
            }
        }

        // Try to read Applet.cap to get applet AIDs
        if let Ok(mut applet_file) = zip.by_name("Applet.cap") {
            let mut applet_data = Vec::new();
            applet_file.read_to_end(&mut applet_data)?;

            if applet_data.len() >= 2 {
                let count = applet_data[1] as usize;
                let mut offset = 2;

                for _ in 0..count {
                    if offset + 1 >= applet_data.len() {
                        break;
                    }

                    let aid_length = applet_data[offset] as usize;
                    offset += 1;

                    if offset + aid_length <= applet_data.len() {
                        let mut applet_aid = Vec::new();
                        applet_aid.extend_from_slice(&applet_data[offset..offset + aid_length]);
                        info.applet_aids.push(applet_aid);
                        offset += aid_length;
                    }
                }
            }
        }

        Ok(info)
    }
}

/// Information about a CAP file
#[derive(Debug, Clone)]
pub struct CapFileInfo {
    /// Package AID
    pub package_aid: Option<Vec<u8>>,
    /// Applet AIDs
    pub applet_aids: Vec<Vec<u8>>,
    /// Package version (major, minor)
    pub version: Option<(u8, u8)>,
    /// List of files in the CAP
    pub files: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_length() {
        // Test short form
        assert_eq!(LoadCommandStream::encode_length(0x7F), vec![0x7F]);

        // Test long form, 1 byte
        assert_eq!(LoadCommandStream::encode_length(0x80), vec![0x81, 0x80]);
        assert_eq!(LoadCommandStream::encode_length(0xFF), vec![0x81, 0xFF]);

        // Test long form, 2 bytes
        assert_eq!(
            LoadCommandStream::encode_length(0x100),
            vec![0x82, 0x01, 0x00]
        );
        assert_eq!(
            LoadCommandStream::encode_length(0xFFFF),
            vec![0x82, 0xFF, 0xFF]
        );

        // Test long form, 3 bytes
        assert_eq!(
            LoadCommandStream::encode_length(0x10000),
            vec![0x83, 0x01, 0x00, 0x00]
        );
    }
}
