//! Cryptographic operations for GlobalPlatform SCP02 protocol
//!
//! This module provides implementations of the cryptographic operations
//! required for the SCP02 protocol, including key derivation, MAC calculation,
//! and cryptogram verification.

use cbc_mac::Mac;
use cipher::{
    BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit, consts::U16, generic_array::GenericArray,
    typenum,
};
use des::{Des, TdesEee3};

use crate::{Error, Result};

/// Null bytes used as initial IV
pub const NULL_BYTES_8: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

/// Derivation purpose for encryption key
pub const DERIVATION_PURPOSE_ENC: [u8; 2] = [0x01, 0x82];
/// Derivation purpose for MAC key
pub const DERIVATION_PURPOSE_MAC: [u8; 2] = [0x01, 0x01];
/// Derivation purpose for data encryption key (DEK)
pub const DERIVATION_PURPOSE_DEK: [u8; 2] = [0x01, 0x81];
/// Derivation purpose for RMAC key
pub const DERIVATION_PURPOSE_RMAC: [u8; 2] = [0x01, 0x02];

/// Derive a session key from the card key using the sequence number and purpose
///
/// This function implements the SCP02 key derivation mechanism.
///
/// # Arguments
///
/// * `card_key` - The card key (16 bytes)
/// * `seq` - The sequence number (2 bytes)
/// * `purpose` - The derivation purpose (2 bytes)
///
/// # Returns
///
/// The derived key (16 bytes)
pub fn derive_key(card_key: [u8; 16], seq: [u8; 2], purpose: [u8; 2]) -> Result<[u8; 16]> {
    // Resize the key to 24 bytes for 3DES (Triple DES)
    let key24 = resize_key_24(card_key.as_slice());

    // Create derivation data
    let mut derivation_data = [0u8; 16];
    derivation_data[0..2].copy_from_slice(&purpose);
    derivation_data[2..4].copy_from_slice(&seq);

    // Convert derivation data to blocks
    let mut blocks = [
        GenericArray::clone_from_slice(&derivation_data[0..8]),
        GenericArray::clone_from_slice(&derivation_data[8..16]),
    ];

    // Create CBC mode encryptor with NULL_BYTES_8 as IV
    let iv = GenericArray::clone_from_slice(&NULL_BYTES_8);

    // Encrypt the blocks in CBC mode
    let mut encryptor = cbc::Encryptor::<des::TdesEde3>::new(&key24.into(), &iv);
    encryptor.encrypt_blocks_mut(&mut blocks);

    // Combine the encrypted blocks into the result
    let mut result = [0u8; 16];
    result[0..8].copy_from_slice(blocks[0].as_slice());
    result[8..16].copy_from_slice(blocks[1].as_slice());

    Ok(result)
}

/// Verify a card cryptogram against the expected value
///
/// # Arguments
///
/// * `enc_key` - The session encryption key
/// * `host_challenge` - The host challenge
/// * `card_challenge` - The card challenge
/// * `card_cryptogram` - The cryptogram provided by the card
///
/// # Returns
///
/// `true` if the cryptogram is valid, `false` otherwise
pub fn verify_cryptogram(
    enc_key: &[u8],
    host_challenge: &[u8],
    card_challenge: &[u8],
    card_cryptogram: &[u8],
) -> Result<bool> {
    // Create buffer for combined data
    let mut combined_data = [0u8; 16]; // Assuming host_challenge and card_challenge are each 8 bytes
    let host_len = host_challenge.len();
    let card_len = card_challenge.len();

    if host_len + card_len > combined_data.len() {
        return Err(Error::InvalidLength {
            expected: combined_data.len(),
            actual: host_len + card_len,
        });
    }

    combined_data[..host_len].copy_from_slice(host_challenge);
    combined_data[host_len..host_len + card_len].copy_from_slice(card_challenge);

    // Use only the filled portion of combined_data
    let data_len = host_len + card_len;
    let mut padded_buf = [0u8; 24];
    let padded_data = append_des_padding_to_slice(&combined_data[..data_len], &mut padded_buf)?;

    let calculated = mac_3des(enc_key, padded_data, &NULL_BYTES_8)?;

    // Compare the calculated MAC with the provided cryptogram
    if calculated.len() != card_cryptogram.len() {
        return Ok(false);
    }

    Ok(calculated.as_slice() == card_cryptogram)
}

/// Calculate a MAC using 3DES in CBC mode
///
/// # Arguments
///
/// * `key` - The key (16 bytes)
/// * `data` - The data to MAC
/// * `iv` - The initialization vector (8 bytes)
///
/// # Returns
///
/// The MAC value (8 bytes)
pub fn mac_3des(key: &[u8], data: &[u8], iv: &[u8]) -> Result<[u8; 8]> {
    let key24 = resize_key_24(key);

    // Initialize a TDES CBC-MAC
    let mut mac = <cbc_mac::CbcMac<TdesEee3> as KeyInit>::new_from_slice(&key24)
        .map_err(|_| Error::Crypto("Failed to initialize 3DES MAC"))?;

    // For custom IV, we need to XOR the first block with IV
    if iv != &NULL_BYTES_8 {
        if data.len() >= 8 {
            // Handle first block with custom IV
            let mut first_block = [0u8; 8];
            first_block.copy_from_slice(&data[..8]);
            for (a, b) in first_block.iter_mut().zip(iv.iter()) {
                *a ^= *b;
            }

            // Update with modified first block then rest of data
            cbc_mac::Mac::update(&mut mac, &first_block);
            cbc_mac::Mac::update(&mut mac, &data[8..]);
        } else {
            // Handle smaller data with custom IV
            let mut first_block = [0u8; 8];
            first_block[..data.len()].copy_from_slice(data);
            for (a, b) in first_block.iter_mut().zip(iv.iter()) {
                *a ^= *b;
            }
            cbc_mac::Mac::update(&mut mac, &first_block);
        }
    } else {
        cbc_mac::Mac::update(&mut mac, data);
    }

    // Finalize and return the MAC as a fixed-size array
    let bytes = mac.finalize().into_bytes();
    let mut result = [0u8; 8];
    result.copy_from_slice(bytes.as_slice());
    Ok(result)
}

/// Calculate a full 3DES MAC for SCP02
///
/// This function implements the specific MAC calculation required for SCP02.
/// It uses single DES for all blocks except the last, which uses 3DES.
///
/// # Arguments
///
/// * `key` - The key (16 bytes)
/// * `data` - The data to MAC
/// * `iv` - The initialization vector (8 bytes)
///
/// # Returns
///
/// The MAC value (8 bytes)
pub fn mac_full_3des(key: &[u8], data: &[u8], iv: &[u8]) -> Result<[u8; 8]> {
    // Padded data buffer (max size consideration)
    let mut padded_buf = [0u8; 256]; // Choose appropriate size based on max expected input
    let padded_data = append_des_padding_to_slice(data, &mut padded_buf)?;

    // For SCP02, we need a specialized MAC algorithm (single DES for all blocks except last)
    // This is custom logic that we have to implement manually
    let des_key8 = resize_key_8_to_array(key);
    let des3_key24 = resize_key_24(key);

    let des_cipher = Des::new_from_slice(&des_key8)
        .map_err(|_| Error::Crypto("Failed to initialize DES cipher"))?;

    let des3_cipher = TdesEee3::new_from_slice(&des3_key24)
        .map_err(|_| Error::Crypto("Failed to initialize 3DES cipher"))?;

    let mut current_iv = [0u8; 8];
    current_iv.copy_from_slice(iv);

    // If data is longer than 8 bytes, process all but the last block with single DES
    if padded_data.len() > 8 {
        let length = padded_data.len() - 8;

        // Process all blocks except the last one with single DES
        for chunk in padded_data[..length].chunks(8) {
            // Create a block with the chunk data
            let mut block = GenericArray::default();
            if chunk.len() < 8 {
                block[..chunk.len()].copy_from_slice(chunk);
            } else {
                block.copy_from_slice(chunk);
            }

            // XOR with current IV
            for (a, b) in block.iter_mut().zip(current_iv.iter()) {
                *a ^= *b;
            }

            // Encrypt with single DES
            des_cipher.encrypt_block(&mut block);

            // Update IV for next block
            current_iv.copy_from_slice(&block);
        }
    }

    // Process the last block with 3DES
    let last_block_start = padded_data.len() - 8;
    let mut last_block = GenericArray::default();
    last_block.copy_from_slice(&padded_data[last_block_start..]);

    // XOR with current IV
    for (a, b) in last_block.iter_mut().zip(current_iv.iter()) {
        *a ^= *b;
    }

    // Encrypt with 3DES
    des3_cipher.encrypt_block(&mut last_block);

    // Return the final MAC
    let mut result = [0u8; 8];
    result.copy_from_slice(&last_block);
    Ok(result)
}

/// Encrypt an ICV (Initial Chaining Vector) for SCP02
///
/// # Arguments
///
/// * `mac_key` - The MAC key (16 bytes)
/// * `icv` - The ICV to encrypt (8 bytes)
///
/// # Returns
///
/// The encrypted ICV (8 bytes)
pub fn encrypt_icv(mac_key: &[u8], icv: &[u8]) -> Result<[u8; 8]> {
    let key8 = resize_key_8_to_array(mac_key);

    let cipher =
        Des::new_from_slice(&key8).map_err(|_| Error::Crypto("Failed to initialize DES cipher"))?;

    // Encrypt the ICV with DES in ECB mode
    let mut block = GenericArray::default();
    if icv.len() < 8 {
        block[..icv.len()].copy_from_slice(icv);
    } else {
        block.copy_from_slice(&icv[..8]);
    }

    cipher.encrypt_block(&mut block);

    let mut result = [0u8; 8];
    result.copy_from_slice(&block);
    Ok(result)
}

/// Append DES padding to data
///
/// This adds a 0x80 byte followed by zeros to make the data length a multiple of 8.
///
/// # Arguments
///
/// * `data` - The data to pad
///
/// # Returns
///
/// The padded data
pub fn append_des_padding(data: &[u8]) -> Vec<u8> {
    let block_size = 8;
    let padding_size = block_size - (data.len() % block_size);
    let mut padded = Vec::with_capacity(data.len() + padding_size);
    padded.extend_from_slice(data);
    padded.push(0x80);

    // Add remaining zeros
    for _ in 1..padding_size {
        padded.push(0);
    }

    padded
}

/// Append DES padding to data slice and write to output buffer
///
/// This adds a 0x80 byte followed by zeros to make the data length a multiple of 8.
///
/// # Arguments
///
/// * `data` - The data to pad
/// * `output` - Buffer to write padded data to
///
/// # Returns
///
/// A slice of the output buffer containing the padded data
pub fn append_des_padding_to_slice<'a>(data: &[u8], output: &'a mut [u8]) -> Result<&'a [u8]> {
    let block_size = 8;
    let padding_size = block_size - (data.len() % block_size);
    let total_size = data.len() + padding_size;

    if total_size > output.len() {
        return Err(Error::InvalidLength {
            expected: output.len(),
            actual: total_size,
        });
    }

    // Copy input data
    output[..data.len()].copy_from_slice(data);

    // Add padding byte 0x80
    output[data.len()] = 0x80;

    // Zero out the rest
    for i in (data.len() + 1)..total_size {
        output[i] = 0;
    }

    Ok(&output[..total_size])
}

/// Resize a 16-byte key to 24 bytes for 3DES
///
/// This copies the first 8 bytes to the end of the key.
///
/// # Arguments
///
/// * `key` - The 16-byte key
///
/// # Returns
///
/// A 24-byte key in a static buffer
pub fn resize_key_24(key: &[u8]) -> [u8; 24] {
    let mut result = [0u8; 24];
    result[..16].copy_from_slice(&key[0..16]);
    result[16..24].copy_from_slice(&key[0..8]);
    result
}

/// Resize a key to 8 bytes for DES into an array
///
/// # Arguments
///
/// * `key` - The key
///
/// # Returns
///
/// An 8-byte key array
pub fn resize_key_8_to_array(key: &[u8]) -> [u8; 8] {
    let mut result = [0u8; 8];
    result.copy_from_slice(&key[0..8]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_derive_key() {
        let card_key = hex!("404142434445464748494a4b4c4d4e4f");
        let seq = hex!("0065");

        let enc_key = derive_key(
            card_key.try_into().unwrap(),
            seq.try_into().unwrap(),
            DERIVATION_PURPOSE_ENC,
        )
        .unwrap();

        assert_eq!(enc_key, hex!("85e72aaf47874218a202bf5ef891dd21"));
    }

    #[test]
    fn test_resize_key_24() {
        let key = hex!("404142434445464748494a4b4c4d4e4f");
        let resized = resize_key_24(&key);

        assert_eq!(
            resized,
            hex!("404142434445464748494a4b4c4d4e4f4041424344454647")
        );
    }

    #[test]
    fn test_append_des_padding() {
        let data = hex!("aabb");
        let padded = append_des_padding(&data);

        assert_eq!(padded, hex!("aabb800000000000"));

        let data = hex!("01020304050607");
        let padded = append_des_padding(&data);

        assert_eq!(padded, hex!("0102030405060780"));

        let data = hex!("0102030405060708");
        let padded = append_des_padding(&data);

        assert_eq!(padded, hex!("01020304050607088000000000000000"));
    }

    #[test]
    fn test_verify_cryptogram() {
        let enc_key = hex!("16b5867ff50be7239c2bf1245b83a362");
        let host_challenge = hex!("32da078d7aac1cff");
        let card_challenge = hex!("007284f64a7d6465");
        let card_cryptogram = hex!("05c4bb8a86014e22");

        let result =
            verify_cryptogram(&enc_key, &host_challenge, &card_challenge, &card_cryptogram)
                .unwrap();
        assert!(result);
    }

    #[test]
    fn test_mac_3des() {
        let key = hex!("16b5867ff50be7239c2bf1245b83a362");
        let data = hex!("32da078d7aac1cff007284f64a7d64658000000000000000");
        let result = mac_3des(&key, &data, &NULL_BYTES_8).unwrap();

        assert_eq!(result, hex!("05c4bb8a86014e22"));
    }

    #[test]
    fn test_mac_full_3des() {
        let key = hex!("5b02e75ad63190aece0622936f11abab");
        let data = hex!("8482010010810b098a8fbb88da");
        let result = mac_full_3des(&key, &data, &NULL_BYTES_8).unwrap();

        assert_eq!(result, hex!("5271d7174a5a166a"));
    }
}
