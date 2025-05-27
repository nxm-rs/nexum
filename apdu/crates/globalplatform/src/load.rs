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

        // Find all files in the archive
        let mut all_file_paths = Vec::new();
        for i in 0..zip.len() {
            if let Ok(file) = zip.by_index(i) {
                all_file_paths.push(file.name().to_string());
            }
        }

        // Function to find a file with a specific suffix in the archive
        let find_file = |suffix: &str| -> Option<String> {
            all_file_paths
                .iter()
                .find(|name| name.ends_with(suffix))
                .cloned()
        };

        // Load all required components
        for file_name in INTERNAL_FILES {
            let cap_suffix = format!("/{}.cap", file_name);
            if let Some(path) = find_file(&cap_suffix) {
                if let Ok(mut file) = zip.by_name(&path) {
                    let mut data = Vec::new();
                    file.read_to_end(&mut data)?;
                    files.insert(*file_name, data);
                }
            } else {
                // Try without .cap extension
                if let Some(path) = find_file(&format!("/{}", file_name)) {
                    if let Ok(mut file) = zip.by_name(&path) {
                        let mut data = Vec::new();
                        file.read_to_end(&mut data)?;
                        files.insert(*file_name, data);
                    }
                }
            }
        }

        // Encode the files into the load file data block format
        let data = Self::encode_files_data(&files)?;
        let blocks_count = data.len().div_ceil(BLOCK_SIZE); // ceiling division

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
            vec![length as u8]
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
    pub const fn blocks_count(&self) -> usize {
        self.blocks_count
    }

    /// Get the current block index
    pub const fn current_block(&self) -> usize {
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
            applet_names: Vec::new(),
            version: None,
            files: Vec::new(),
        };

        // List all files in the archive
        for i in 0..zip.len() {
            if let Ok(file) = zip.by_index(i) {
                info.files.push(file.name().to_string());
            }
        }

        // Primary approach: Parse MANIFEST.MF first
        if let Ok(mut manifest_file) = zip.by_name("META-INF/MANIFEST.MF") {
            let mut manifest_data = String::new();
            manifest_file.read_to_string(&mut manifest_data)?;

            parse_manifest(&manifest_data, &mut info)?;
        }

        // Supplement with data from applet.xml if available
        if let Ok(mut xml_file) = zip.by_name("APPLET-INF/applet.xml") {
            let mut xml_data = String::new();
            xml_file.read_to_string(&mut xml_data)?;

            enhance_with_applet_xml(&xml_data, &mut info)?;
        }

        // As a last resort, try the raw CAP file components if we're still missing info
        if info.package_aid.is_none() {
            // Function to find a file with a specific suffix in the archive
            let find_file = |suffix: &str| -> Option<String> {
                info.files
                    .iter()
                    .find(|name| name.ends_with(suffix))
                    .cloned()
            };

            // Process Header.cap file
            if let Some(header_name) = find_file("/Header.cap") {
                if let Ok(mut header_file) = zip.by_name(&header_name) {
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
            }

            // Process Applet.cap file if we still don't have applet AIDs
            if info.applet_aids.is_empty() {
                if let Some(applet_name) = find_file("/Applet.cap") {
                    if let Ok(mut applet_file) = zip.by_name(&applet_name) {
                        let mut applet_data = Vec::new();
                        applet_file.read_to_end(&mut applet_data)?;

                        if applet_data.len() >= 2 {
                            let count = applet_data[1] as usize;
                            let mut offset = 2;

                            for i in 0..count {
                                if offset + 1 >= applet_data.len() {
                                    break;
                                }

                                let aid_length = applet_data[offset] as usize;
                                offset += 1;

                                if offset + aid_length <= applet_data.len() {
                                    let mut applet_aid = Vec::new();
                                    applet_aid.extend_from_slice(
                                        &applet_data[offset..offset + aid_length],
                                    );
                                    info.applet_aids.push(applet_aid);
                                    info.applet_names.push(format!("Applet {}", i + 1));
                                    offset += aid_length;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(info)
    }
}

/// Helper function to parse the MANIFEST.MF file
fn parse_manifest(manifest_data: &str, info: &mut CapFileInfo) -> Result<()> {
    // Parse package AID
    if let Some(package_aid_line) = manifest_data
        .lines()
        .find(|line| line.starts_with("Java-Card-Package-AID:"))
    {
        // The line format is typically "Java-Card-Package-AID: 0xa0:0x00:0x00:0x08:0x04:0x00:0x01"
        // We need everything after the colon and space
        if let Some(after_colon) = package_aid_line.find(": ") {
            let aid_part = &package_aid_line[after_colon + 2..];
            if let Ok(aid_bytes) = parse_aid_bytes(aid_part) {
                info.package_aid = Some(aid_bytes);
            }
        }
    }

    // Parse package version
    if let Some(version_line) = manifest_data
        .lines()
        .find(|line| line.starts_with("Java-Card-Package-Version:"))
    {
        if let Some(after_colon) = version_line.find(": ") {
            let version_str = &version_line[after_colon + 2..];

            // Try to parse version in format "x.y"
            let parts: Vec<&str> = version_str.split('.').collect();
            if parts.len() >= 2 {
                if let (Ok(major), Ok(minor)) =
                    (parts[0].trim().parse::<u8>(), parts[1].trim().parse::<u8>())
                {
                    info.version = Some((major, minor));
                }
            }
        }
    }

    // Parse applet AIDs and names
    let mut applet_index = 1;
    loop {
        let aid_key = format!("Java-Card-Applet-{}-AID:", applet_index);
        let name_key = format!("Java-Card-Applet-{}-Name:", applet_index);

        let aid_line = manifest_data
            .lines()
            .find(|line| line.starts_with(&aid_key));
        let name_line = manifest_data
            .lines()
            .find(|line| line.starts_with(&name_key));

        if aid_line.is_none() {
            break; // No more applets found
        }

        if let Some(aid_line) = aid_line {
            if let Some(after_colon) = aid_line.find(": ") {
                let aid_part = &aid_line[after_colon + 2..];
                if let Ok(aid_bytes) = parse_aid_bytes(aid_part) {
                    info.applet_aids.push(aid_bytes);

                    // Try to get the applet name
                    if let Some(name_line) = name_line {
                        if let Some(after_colon) = name_line.find(": ") {
                            let name = &name_line[after_colon + 2..];
                            info.applet_names.push(name.trim().to_string());
                        } else {
                            info.applet_names.push(format!("Applet {}", applet_index));
                        }
                    } else {
                        info.applet_names.push(format!("Applet {}", applet_index));
                    }
                }
            }
        }

        applet_index += 1;
    }

    Ok(())
}

/// Helper function to enhance info with display names from applet.xml
fn enhance_with_applet_xml(xml_data: &str, info: &mut CapFileInfo) -> Result<()> {
    // Map of applet AIDs to improved display names
    let mut display_names = std::collections::HashMap::new();

    // Extract applet information
    let mut applet_start = 0;
    while let Some(applet_idx) = xml_data[applet_start..].find("<applet>") {
        let start_pos = applet_start + applet_idx;
        if let Some(end_idx) = xml_data[start_pos..].find("</applet>") {
            let end_pos = start_pos + end_idx + 9; // +9 for "</applet>"
            let applet_section = &xml_data[start_pos..end_pos];

            // Extract applet name
            let mut display_name = String::new();
            if let Some(name_start) = applet_section.find("<display-name>") {
                if let Some(name_end) = applet_section[name_start..].find("</display-name>") {
                    display_name = applet_section[name_start + 14..name_start + name_end]
                        .trim()
                        .to_string();
                }
            }

            // Extract applet class
            let mut class_name = String::new();
            if let Some(class_start) = applet_section.find("<applet-class>") {
                if let Some(class_end) = applet_section[class_start..].find("</applet-class>") {
                    class_name = applet_section[class_start + 14..class_start + class_end]
                        .trim()
                        .to_string();
                }
            }

            // Extract applet AID
            if let Some(aid_start) = applet_section.find("<applet-AID>") {
                if let Some(aid_end) = applet_section[aid_start..].find("</applet-AID>") {
                    let aid_str = &applet_section[aid_start + 12..aid_start + aid_end];

                    // Format: //aid/A000000804/000101
                    if let Some(stripped) = aid_str.strip_prefix("//aid/") {
                        let parts: Vec<&str> = stripped.split('/').collect();
                        if parts.len() >= 2 {
                            let package_part = parts[0];
                            let instance_part = parts[1];

                            // Combine them to form the complete AID
                            let mut aid_hex = String::new();
                            aid_hex.push_str(package_part);
                            aid_hex.push_str(instance_part);

                            if let Ok(aid) = hex::decode(&aid_hex) {
                                // Create a composite display name with both display name and class name
                                let full_name =
                                    if !display_name.is_empty() && !class_name.is_empty() {
                                        format!("{} ({})", display_name, class_name)
                                    } else if !display_name.is_empty() {
                                        display_name
                                    } else if !class_name.is_empty() {
                                        class_name
                                    } else {
                                        String::from("Unknown")
                                    };

                                display_names.insert(hex::encode(&aid), full_name);
                            }
                        }
                    }
                }
            }

            applet_start = end_pos;
        } else {
            break;
        }
    }

    // Update applet names with display names from applet.xml where available
    for i in 0..info.applet_aids.len() {
        if i < info.applet_names.len() {
            let aid_hex = hex::encode(&info.applet_aids[i]);
            if let Some(display_name) = display_names.get(&aid_hex) {
                info.applet_names[i] = display_name.clone();
            }
        }
    }

    Ok(())
}

/// Helper function to parse AID bytes from string like "0xa0:0x00:0x00:0x08:0x04:0x00:0x01"
fn parse_aid_bytes(aid_str: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();

    for part in aid_str.split(':') {
        let part = part.trim();

        // The format in MANIFEST.MF is "0xa0:0x00:0x00:0x08:0x04:0x00:0x01"
        if part.starts_with("0x") && part.len() >= 3 {
            // Parse hex value with 0x prefix
            match u8::from_str_radix(&part[2..], 16) {
                Ok(byte) => bytes.push(byte),
                Err(_) => return Err(Error::CapFile("Invalid AID byte: {}")),
            }
        } else {
            return Err(Error::CapFile(
                "Invalid AID byte format, expected 0x prefix: {}",
            ));
        }
    }

    if bytes.is_empty() {
        return Err(Error::CapFile("Empty AID"));
    }

    Ok(bytes)
}

/// Information about a CAP file
#[derive(Debug, Clone)]
pub struct CapFileInfo {
    /// Package AID
    pub package_aid: Option<Vec<u8>>,
    /// Applet AIDs
    pub applet_aids: Vec<Vec<u8>>,
    /// Applet names corresponding to the AIDs
    pub applet_names: Vec<String>,
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
