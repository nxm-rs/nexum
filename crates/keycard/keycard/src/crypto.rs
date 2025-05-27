use aes::cipher::{
    BlockDecryptMut, BlockEncryptMut, Iv, IvSizeUser, Key, KeyIvInit, KeySizeUser,
    block_padding::Iso7816,
    typenum::{U16, U32},
};
use alloy_primitives::bytes::{Bytes, BytesMut};
use cipher::block_padding::UnpadError;
use generic_array::GenericArray;
use k256::{PublicKey, SecretKey, ecdh::SharedSecret, elliptic_curve::sec1::ToEncodedPoint};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::{Digest, Sha256, Sha512};
use unicode_normalization::UnicodeNormalization;

pub const PAIRING_TOKEN_SALT: &str = "Keycard Pairing Password Salt";

pub type PairingToken = GenericArray<u8, U32>;
pub type Cryptogram = GenericArray<u8, U32>;
pub type Challenge = GenericArray<u8, U32>;
pub(crate) type ApduMeta = GenericArray<u8, U16>;

type Encryptor = cbc::Encryptor<aes::Aes256>;
type Decryptor = cbc::Decryptor<aes::Aes256>;

pub struct KeycardScp;

impl KeySizeUser for KeycardScp {
    type KeySize = U32;
}

impl IvSizeUser for KeycardScp {
    type IvSize = U16;
}

pub fn generate_ecdh_shared_secret(private: &SecretKey, public: &PublicKey) -> SharedSecret {
    k256::elliptic_curve::ecdh::diffie_hellman(private.to_nonzero_scalar(), public.as_affine())
}

/// Verify the cryptogram using the provided challenge and shared secret.
///
/// # Arguments
/// * `challenge` - The challenge used to generate the cryptogram.
/// * `shared_secret` - The shared secret used to generate the cryptogram.
/// * `card_cryptogram` - The cryptogram to be verified.
///
/// # Returns
/// `true` if the cryptogram is valid, `false` otherwise.
pub(crate) fn calculate_cryptogram(
    shared_secret: &PairingToken,
    challenge: &Challenge,
) -> Cryptogram {
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.update(challenge);
    hasher.finalize()
}

/// Perform one-shot encryption using the provided public key and ECDH shared secret.
/// Used for encrypting initialisation data (e.g. PIN/PUK/pairing password).
///
/// # Arguments
/// * `public_key` - The public key of the recipient.
/// * `ecdh_shared_secret` - The ECDH shared secret.
/// * `data` - The data to be encrypted.
///
/// # Returns
/// The encrypted data.
pub(crate) fn one_shot_encrypt(
    public_key: &PublicKey,
    ecdh_shared_secret: &SharedSecret,
    data: &mut BytesMut,
) -> Bytes {
    let mut iv = Iv::<KeycardScp>::default();
    rand::rng().fill_bytes(&mut iv);

    let msg_len = prepare_padding(data);
    // SAFETY: The data is padded to a multiple of 16 bytes, so it is safe to use with the Encryptor.
    let ciphertext = Encryptor::new(ecdh_shared_secret.raw_secret_bytes(), &iv)
        .encrypt_padded_mut::<Iso7816>(data, msg_len)
        .unwrap();

    let pub_key_data = public_key.to_encoded_point(false);
    let mut buf = BytesMut::new();
    buf.extend(&[pub_key_data.len() as u8]);
    buf.extend(pub_key_data.as_bytes());
    // The last 16 bytes of the ciphertext are the IV
    buf.extend(&iv);
    buf.extend(ciphertext);

    buf.into()
}

/// Derive session keys from the shared secret, pairing key, and challenge.
///
/// Keys are derived in accordance with the Keycard Secure Channel protocol specification:
/// (K(E) | K(M)) = H(shared_secret_key | pairing_key | challenge)
///
/// # Arguments
///
/// * `shared_secret_key` - The shared secret key.
/// * `pairing_key` - The pairing key.
/// * `challenge` - The challenge.
///
/// # Returns
///
/// A tuple containing the encryption key and the MAC key.
pub(crate) fn derive_session_keys(
    secret: SharedSecret,
    pairing_key: &Key<KeycardScp>,
    challenge: &Challenge,
) -> (Key<KeycardScp>, Key<KeycardScp>) {
    let mut hasher = Sha512::new();
    hasher.update(secret.raw_secret_bytes());
    hasher.update(pairing_key);
    hasher.update(challenge);
    let data = hasher.finalize();

    let enc_key = Key::<KeycardScp>::clone_from_slice(&data[0..32]);
    let mac_key = Key::<KeycardScp>::clone_from_slice(&data[32..64]);

    (enc_key, mac_key)
}

/// Encrypt data using the provided key and IV, padding it in ISO 7816 format.
///
/// # Arguments
///
/// * `data` - The data to encrypt.
/// * `enc_key` - The key to use for encryption.
/// * `iv` - The IV to use for encryption.
///
/// # Returns
///
/// The encrypted data as a `Bytes`.
pub(crate) fn encrypt_data(
    data: &mut BytesMut,
    enc_key: &Key<KeycardScp>,
    iv: &Iv<KeycardScp>,
) -> Bytes {
    let msg_len = prepare_padding(data);
    // SAFETY: The data is padded to a multiple of 16 bytes, so it is safe to use with the Encryptor.
    let encrypted = Encryptor::new(enc_key, iv)
        .encrypt_padded_mut::<Iso7816>(data, msg_len)
        .unwrap();
    Bytes::copy_from_slice(encrypted)
}

/// Decrypt data using the provided key and IV assuming the data is padded in ISO 7816 format.
///
/// # Arguments
///
/// * `data` - The data to decrypt.
/// * `enc_key` - The key to use for decryption.
/// * `iv` - The IV to use for decryption.
///
/// # Returns
///
/// The decrypted data as a `Bytes`.
pub(crate) fn decrypt_data(
    data: &mut BytesMut,
    enc_key: &Key<KeycardScp>,
    iv: &Iv<KeycardScp>,
) -> Result<Bytes, UnpadError> {
    let decrypted = Decryptor::new(enc_key, iv).decrypt_padded_mut::<Iso7816>(data)?;

    Ok(BytesMut::from(decrypted).into())
}

/// Calculate the MAC for the given data and encrypt it using the provided key and IV.
///
/// # Arguments
///
/// * `meta` - The APDU metadata.
/// * `data` - The data to calculate the MAC for.
/// * `mac_key` - The key to use for calculating the MAC.
///
/// # Returns
///
/// The calculated MAC as an IV.
pub(crate) fn calculate_mac(
    meta: &ApduMeta,
    data: &Bytes,
    mac_key: &Key<KeycardScp>,
) -> Iv<KeycardScp> {
    let iv = Iv::<KeycardScp>::default();

    // Concatenate meta and data
    let mut buf = BytesMut::new();
    buf.extend_from_slice(meta.as_slice());
    buf.extend_from_slice(data);

    let msg_len = prepare_padding(&mut buf);
    // SAFETY: The data is padded to a multiple of 16 bytes, so it is safe to use with the Encryptor.
    let ciphertext = Encryptor::new(mac_key, &iv)
        .encrypt_padded_mut::<Iso7816>(&mut buf, msg_len)
        .unwrap();

    *Iv::<KeycardScp>::from_slice(&ciphertext[ciphertext.len() - 32..ciphertext.len() - 16])
}

/// Generate a pairing token using PBKDF2-HMAC-SHA256 in accordance with the Keycard specification.
///
/// # Arguments
///
/// * `password` - The password to use for generating the token.
///
/// # Returns
///
/// A `PairingToken` representing the generated token.
pub(crate) fn generate_pairing_token(password: &str) -> PairingToken {
    let password = password.nfkd().collect::<String>();
    let salt = PAIRING_TOKEN_SALT.nfkd().collect::<String>();

    let mut token = PairingToken::default();
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 50000, &mut token);

    token
}

// A utility function to ensure that the data is padded to a multiple of 16 bytes.
fn prepare_padding(data: &mut BytesMut) -> usize {
    let len = data.len();
    data.resize(len + 16 - len % 16, 0);

    len
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::bytes;
    use k256::SecretKey;

    #[test]
    fn test_ecdh() {
        // Generate two private keys
        let pk1 = SecretKey::random(&mut rand_v8::thread_rng());
        let pk2 = SecretKey::random(&mut rand_v8::thread_rng());

        // Get their public keys
        let pub1 = pk1.public_key();
        let pub2 = pk2.public_key();

        // Generate shared secrets
        let shared_secret1 = generate_ecdh_shared_secret(&pk1, &pub2);
        let shared_secret2 = generate_ecdh_shared_secret(&pk2, &pub1);

        assert_eq!(
            shared_secret1.raw_secret_bytes(),
            shared_secret2.raw_secret_bytes()
        );
    }

    #[test]
    fn test_derive_session_keys() {
        let secret = bytes!("B410E816DA313545151807E25A830201FA389913A977066AB0C6DE0E8631E400");
        let pairing_key =
            bytes!("544FF0B9B0737E4BFC4ECDFCE09F522B837051BBE4FFCEC494FA420D8525670E");
        let card_data = bytes!(
            "1D7C033E75E10EC578AB538F69F1B02538571BA3831441F1649E3F24B5B3E3E71D7BC2D6A3D02FC8CB2FBB3FD8711BB5"
        );

        let shared_secret_key: Key<KeycardScp> =
            Key::<KeycardScp>::clone_from_slice(secret.to_vec().as_slice());

        let challenge = Challenge::from_slice(&card_data[..32]);
        let iv = Iv::<KeycardScp>::from_slice(&card_data[32..48]);

        let (enc_key, mac_key) = derive_session_keys(
            SharedSecret::from(shared_secret_key),
            pairing_key.as_ref().try_into().unwrap(),
            challenge,
        );

        let expected_iv = bytes!("1D7BC2D6A3D02FC8CB2FBB3FD8711BB5");
        let expected_enc_key =
            bytes!("4FF496554C01BAE0A52323E3481B448C99D43982118D95C6918FE0354D224B90");
        let expected_mac_key =
            bytes!("185811013138EA1B4FFDBBFA7343EF2DBE3E54C2C231885E867F792448AC2FE5");

        assert_eq!(expected_iv.as_ref(), iv.as_slice());
        assert_eq!(expected_enc_key.as_ref(), enc_key.as_slice());
        assert_eq!(expected_mac_key.as_ref(), mac_key.as_slice());
    }

    #[test]
    fn test_encrypt_data() {
        let data = bytes!("A8A686D0E3290459BCB36088A8FD04A76BF13283BE4B1EAE2E1248EF609F94DC");
        let enc_key = bytes!("44D689AB4B18206F7EEE5439FB9A71A8A617406BA5259728D1EBC2786D24896C");
        let iv = bytes!("9D3EF41EF1D221DD98A54AD5470F58F2");

        let encrypted_data = encrypt_data(
            &mut BytesMut::from(data.as_ref()),
            enc_key.as_ref().try_into().unwrap(),
            iv.as_ref().try_into().unwrap(),
        );

        let expected = bytes!(
            "FFB41FED5F71A2B57A6AE62D5D5ECD1C12616F6464637DD0A7A930920ACBA55867A7E12CC4F06B089AF34FF4ED4BAB08"
        );
        assert_eq!(expected, encrypted_data);
    }

    #[test]
    fn test_decrypt_data() {
        let enc_data = bytes!(
            "73B58B66372E3446E14A9F54BA59666DB432E9DD87D24F9B0525180EE52DA2106E0C70EED7CD42B5B313E4443D6AC90D"
        );
        let enc_key = bytes!("D93D8E6164196D5C5B5F84F10E4B90D98F8D282ED145513ED666AA55C9871E79");
        let iv = bytes!("F959B1220333046D3C47D61B1E1B891B");

        let mut enc_data = bytes::BytesMut::from(enc_data.as_ref());
        let data = decrypt_data(
            &mut enc_data,
            enc_key.as_ref().try_into().unwrap(),
            iv.as_ref().try_into().unwrap(),
        )
        .unwrap();

        let expected =
            bytes!("2E21F9F2B2C2CC9038D518A5C6B490613E7955BD19D19108B77786986B7ABFE69000");
        assert_eq!(expected, data);
    }
}
