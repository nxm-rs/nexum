//! Cryptographic operations for GlobalPlatform SCP02 protocol
//!
//! This module provides implementations of the cryptographic operations
//! required for the SCP02 protocol, including key derivation, MAC calculation,
//! and cryptogram verification.

use block_padding::{Iso7816, Padding, array::Array};
use cbc_mac::{CbcMac, Mac};
use cipher::{
    BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit,
    consts::{U24, U256},
    generic_array::GenericArray,
};
use des::{Des, TdesEde3};

use crate::{Error, Result};

/// Null bytes used as initial IV
pub const NULL_BYTES_8: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

/// Derivation purpose for encryption key
pub const DERIVATION_PURPOSE_ENC: [u8; 2] = [0x01, 0x82];
/// Derivation purpose for MAC key
pub const DERIVATION_PURPOSE_MAC: [u8; 2] = [0x01, 0x01];

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
pub fn derive_key(card_key: &[u8; 16], seq: &[u8; 2], purpose: &[u8; 2]) -> Result<[u8; 16]> {
    // Resize the key to 24 bytes for 3DES (Triple DES)
    let key24 = resize_key_24(card_key.as_slice());

    // Create derivation data
    let mut derivation_data = [0u8; 16];
    derivation_data[0..2].copy_from_slice(purpose);
    derivation_data[2..4].copy_from_slice(seq);

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
    enc_key: &[u8; 16],
    sequence_counter: &[u8; 2],
    card_challenge: &[u8; 6],
    host_challenge: &[u8; 8],
    card_cryptogram: &[u8; 8],
) -> Result<bool> {
    Ok(calculate_cryptogram(
        enc_key,
        sequence_counter,
        card_challenge,
        host_challenge,
        false,
    )? == *card_cryptogram)
}

/// Calculate a cryptogram using 3DES in CBC mode
///
/// # Arguments
///
/// * `enc_key` - The encryption key (16 bytes)
/// * `host_challenge` - The host challenge (8 bytes)
/// * `sequence_counter` - The sequence counter (2 bytes)
/// * `card_challenge` - The card challenge (6 bytes)
///
/// # Returns
///
/// The calculated cryptogram (8 bytes)
pub fn calculate_cryptogram(
    enc_key: &[u8; 16],
    sequence_counter: &[u8; 2],
    card_challenge: &[u8; 6],
    host_challenge: &[u8; 8],
    for_host: bool,
) -> Result<[u8; 8]> {
    let mut block: Array<u8, U24> = [0u8; 24].into();

    if for_host {
        // Host cryptogram order for EXTERNAL AUTHENTICATE: sequence counter + card challenge + host challenge
        block[0..2].copy_from_slice(sequence_counter);
        block[2..8].copy_from_slice(card_challenge);
        block[8..16].copy_from_slice(host_challenge);
    } else {
        // Card cryptogram order for INITIALIZE UPDATE: host challenge + sequence counter + card challenge
        block[0..8].copy_from_slice(host_challenge);
        block[8..10].copy_from_slice(sequence_counter);
        block[10..16].copy_from_slice(card_challenge);
    }

    Iso7816::pad(&mut block, 16);

    mac_3des(enc_key, &NULL_BYTES_8, &block)
}

/// Calculate a MAC using 3DES in CBC mode
///
/// # Arguments
///
/// * `key` - The key (16 bytes)
/// * `iv` - The initialization vector (8 bytes)
/// * `data` - The data to MAC
///
/// # Returns
///
/// The MAC value (8 bytes)
pub fn mac_3des(key: &[u8; 16], iv: &[u8; 8], data: &[u8]) -> Result<[u8; 8]> {
    let key24 = resize_key_24(key.as_slice());

    // We need to implement CBC-MAC with a custom IV manually
    // since most MAC implementations use a zero IV by default
    let mut mac = <CbcMac<TdesEde3> as Mac>::new_from_slice(&key24).unwrap();

    // Set IV through the first block processing
    // If data is less than a block, we need to pad it
    let mut first_block = [0u8; 8];
    let first_len = std::cmp::min(data.len(), 8);
    first_block[..first_len].copy_from_slice(&data[..first_len]);

    // XOR first block with IV
    for i in 0..8 {
        first_block[i] ^= iv[i];
    }

    // Process first block manually
    mac.update(&first_block);

    // Process the rest of the data
    if data.len() > 8 {
        mac.update(&data[8..]);
    }

    // Finalize and return the MAC as a fixed-size array
    Ok(mac.finalize().into_bytes().into())
}

/// Calculate a full 3DES MAC for SCP02
///
/// This function implements the specific MAC calculation required for SCP02.
/// It uses single DES for all blocks except the last, which uses 3DES.
///
/// # Arguments
///
/// * `key` - The key (16 bytes)
/// * `iv` - The initialization vector (8 bytes)
/// * `data` - The data to MAC
///
/// # Returns
///
/// The MAC value (8 bytes)
pub fn mac_full_3des(key: &[u8], iv: &[u8], data: &[u8]) -> Result<[u8; 8]> {
    // Calculate padded length (includes padding bytes)
    let padding_bytes = if data.len() % 8 == 0 {
        8
    } else {
        8 - (data.len() % 8)
    };
    let padded_len = data.len() + padding_bytes;

    // Create a buffer with at least the minimum needed size
    let mut block = Array::<u8, U256>::default();
    block[..data.len()].copy_from_slice(data);

    // Apply ISO 7816 padding using the provided function
    Iso7816::pad(&mut block, data.len());

    // Extract only the bytes we need for MAC calculation
    let padded_data = &block[..padded_len];

    // For SCP02, we need a specialized MAC algorithm (single DES for all blocks except last)
    // This is custom logic that we have to implement manually
    let des_key8 = &key[..8];
    let des3_key24 = resize_key_24(key);

    let des_cipher = Des::new_from_slice(&des_key8)
        .map_err(|_| Error::Crypto("Failed to initialize DES cipher"))?;

    let des3_cipher = TdesEde3::new_from_slice(&des3_key24)
        .map_err(|_| Error::Crypto("Failed to initialize 3DES cipher"))?;

    let mut current_iv = [0u8; 8];
    current_iv.copy_from_slice(iv);

    // If data is longer than 8 bytes, process all but the last block with single DES
    if padded_len > 8 {
        let length = padded_len - 8;

        // Process all blocks except the last one with single DES
        for chunk in padded_data[..length].chunks(8) {
            // Create a block with the chunk data
            let mut block = GenericArray::default();
            block.copy_from_slice(chunk);

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
    let last_block_start = padded_len - 8;
    let mut last_block = GenericArray::default();
    last_block.copy_from_slice(&padded_data[last_block_start..last_block_start + 8]);

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
pub fn encrypt_icv(mac_key: &[u8; 16], icv: &[u8; 8]) -> Result<[u8; 8]> {
    let key = GenericArray::from_slice(&mac_key[..8]);
    let mut mac = <CbcMac<Des> as Mac>::new(&key);

    // Process the icv
    mac.update(&icv.as_ref());

    // Finalize and return the MAC as a fixed-size array
    Ok(mac.finalize().into_bytes().into())
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

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_derive_key() {
        let card_key = hex!("404142434445464748494a4b4c4d4e4f");
        let seq = hex!("0065");

        let enc_key = derive_key(&card_key, &seq, &DERIVATION_PURPOSE_ENC).unwrap();

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
    fn test_verify_cryptogram() {
        let enc_key = hex!("16b5867ff50be7239c2bf1245b83a362");
        let host_challenge = hex!("32da078d7aac1cff");
        let sequence_counter = hex!("0072");
        let card_challenge = hex!("84f64a7d6465");
        let card_cryptogram = hex!("05c4bb8a86014e22");

        let result = verify_cryptogram(
            &enc_key,
            &sequence_counter,
            &card_challenge,
            &host_challenge,
            &card_cryptogram,
        )
        .unwrap();
        assert!(result);
    }

    #[test]
    fn test_mac_3des() {
        let key = hex!("16b5867ff50be7239c2bf1245b83a362");
        let data = hex!("32da078d7aac1cff007284f64a7d64658000000000000000");
        let result = mac_3des(&key, &&NULL_BYTES_8, &data).unwrap();

        assert_eq!(result, hex!("05c4bb8a86014e22"));
    }

    #[test]
    fn test_mac_full_3des() {
        let key = hex!("5b02e75ad63190aece0622936f11abab");
        let data = hex!("8482010010810b098a8fbb88da");
        let result = mac_full_3des(&key, &NULL_BYTES_8, &data).unwrap();

        assert_eq!(result, hex!("5271d7174a5a166a"));
    }
}
