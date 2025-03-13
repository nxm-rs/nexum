//! Cryptographic operations for GlobalPlatform SCP02 protocol
//!
//! This module provides implementations of the cryptographic operations
//! required for the SCP02 protocol, including key derivation, MAC calculation,
//! and cryptogram verification.

use block_padding::{Iso7816, Padding, RawPadding, array::Array};
use cbc_mac::{CbcMac, Mac};
use cipher::{
    BlockEncrypt, BlockEncryptMut, Iv, IvSizeUser, Key, KeyInit, KeyIvInit, KeySizeUser,
    consts::{U8, U16, U256},
    generic_array::GenericArray,
};
use des::{Des, TdesEde3};

use crate::Result;

pub type Purpose = [u8; 2];
pub type SequenceCounter = [u8; 2];
pub type CardChallenge = [u8; 6];
pub type HostChallenge = [u8; 8];
pub type Cryptogram = [u8; 8];
pub type Scp02Mac = [u8; 8];

/// Derivation purpose for encryption key
pub const DERIVATION_ENC: Purpose = [0x01, 0x82];
/// Derivation purpose for MAC key
pub const DERIVATION_MAC: Purpose = [0x01, 0x01];

/// Placeholder struct for defining SCP02 cryptographic parameters
#[allow(missing_debug_implementations)]
pub struct Scp02;

impl KeySizeUser for Scp02 {
    type KeySize = U16;
}

impl IvSizeUser for Scp02 {
    type IvSize = U8;
}

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
pub fn derive_key(
    card_key: &Key<Scp02>,
    seq: &SequenceCounter,
    purpose: &Purpose,
) -> Result<Key<Scp02>> {
    // Create derivation data
    let mut blocks = [GenericArray::default(), GenericArray::default()];
    blocks[0][0..2].copy_from_slice(purpose);
    blocks[0][2..4].copy_from_slice(seq);

    // Prepare 3DES key and zero IV
    let key = resize_key(&card_key);
    let iv = GenericArray::default();

    // Encrypt the blocks in CBC mode
    let mut encryptor = cbc::Encryptor::<des::TdesEde3>::new(&key, &iv);
    encryptor.encrypt_blocks_mut(&mut blocks);

    // Convert the encrypted blocks to result key
    let mut result = Key::<Scp02>::default();
    result[0..8].copy_from_slice(blocks[0].as_slice());
    result[8..16].copy_from_slice(blocks[1].as_slice());

    Ok(result)
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
    enc_key: &Key<Scp02>,
    sequence_counter: &SequenceCounter,
    card_challenge: &CardChallenge,
    host_challenge: &HostChallenge,
    for_host: bool,
) -> Cryptogram {
    // Create exactly 3 blocks (24 bytes) for data
    let mut blocks = [GenericArray::default(); 3];

    if for_host {
        // Host cryptogram order: sequence counter + card challenge + host challenge
        blocks[0][0..2].copy_from_slice(sequence_counter);
        blocks[0][2..8].copy_from_slice(card_challenge);
        blocks[1][0..8].copy_from_slice(host_challenge);
    } else {
        // Card cryptogram order: host challenge + sequence counter + card challenge
        blocks[0][0..8].copy_from_slice(host_challenge);
        blocks[1][0..2].copy_from_slice(sequence_counter);
        blocks[1][2..8].copy_from_slice(card_challenge);
    }

    // Pad and Calculate MAC with zero IV
    Iso7816::raw_pad(&mut blocks[2], 0);
    let mut cipher = cbc::Encryptor::<TdesEde3>::new(&resize_key(enc_key), &Default::default());

    // Encrypt blocks in place and return the last encrypted block as the MAC
    cipher.encrypt_blocks_mut(&mut blocks);
    blocks[blocks.len() - 1].into()
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
pub fn mac_full_3des(key: &Key<Scp02>, iv: &Iv<Scp02>, data: &[u8]) -> Scp02Mac {
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
    let des3_key24 = resize_key(&key);

    // This is safe as otherwise the direct assignment above would have paniced.
    let des_cipher = Des::new_from_slice(&des_key8).unwrap();
    let des3_cipher = TdesEde3::new(&des3_key24);

    let mut current_iv = Iv::<Scp02>::default();
    current_iv.copy_from_slice(iv.as_slice());

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
    last_block.into()
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
pub fn encrypt_icv(mac_key: &Key<Scp02>, icv: &Iv<Scp02>) -> Iv<Scp02> {
    let key = GenericArray::from_slice(&mac_key[..8]);
    let mut mac = <CbcMac<Des> as Mac>::new(&key);
    mac.update(&icv.as_ref());
    mac.finalize().into_bytes().into()
}

/// Resize the SCP02 16-byte key to 24 bytes for 3DES
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
pub fn resize_key(key: &Key<Scp02>) -> Key<TdesEde3> {
    let mut result = Key::<TdesEde3>::default();
    result[..16].copy_from_slice(&key);
    result[16..24].copy_from_slice(&key[..8]);
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
            &Key::<Scp02>::clone_from_slice(&card_key),
            &seq,
            &DERIVATION_ENC,
        )
        .unwrap();

        assert_eq!(enc_key.as_slice(), hex!("85e72aaf47874218a202bf5ef891dd21"));
    }

    #[test]
    fn test_resize_key_24() {
        let key = hex!("404142434445464748494a4b4c4d4e4f");
        let resized = resize_key(&Key::<Scp02>::clone_from_slice(&key));

        assert_eq!(
            resized.as_slice(),
            hex!("404142434445464748494a4b4c4d4e4f4041424344454647")
        );
    }

    #[test]
    fn test_verify_cryptogram() {
        let enc_key = hex!("16b5867ff50be7239c2bf1245b83a362");
        let enc_key = Key::<Scp02>::clone_from_slice(&enc_key);
        let host_challenge = hex!("32da078d7aac1cff");
        let sequence_counter = hex!("0072");
        let card_challenge = hex!("84f64a7d6465");
        let card_cryptogram = hex!("05c4bb8a86014e22");

        let result = calculate_cryptogram(
            &enc_key,
            &sequence_counter,
            &card_challenge,
            &host_challenge,
            false,
        );
        assert_eq!(result, card_cryptogram);
    }

    #[test]
    fn test_mac_full_3des() {
        let key = hex!("5b02e75ad63190aece0622936f11abab");
        let key = Key::<Scp02>::clone_from_slice(&key);
        let data = hex!("8482010010810b098a8fbb88da");
        let result = mac_full_3des(&key, &Default::default(), &data);

        assert_eq!(result, hex!("5271d7174a5a166a"));
    }
}
