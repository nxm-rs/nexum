use crate::Error;
use crate::commands::ExportOption;
use crate::types::Keypair;
use k256::{PublicKey, SecretKey};

/// Represents different types of keys that can be exported from the keycard
#[derive(Debug, Clone)]
pub enum ExportedKey {
    /// Both public and private key components
    Complete {
        /// The private key
        private_key: SecretKey,
        /// The public key (may be None if the card omitted it for performance)
        public_key: Option<PublicKey>,
    },
    /// Public key only
    PublicOnly(PublicKey),
    /// Extended public key (public key + chain code)
    Extended {
        /// The public key
        public_key: PublicKey,
        /// The chain code used for derivation
        chain_code: Vec<u8>,
    },
}

impl ExportedKey {
    /// Try to convert a Keypair into an ExportedKey based on what was requested
    pub fn try_from_keypair(keypair: Keypair, requested: ExportOption) -> Result<Self, Error> {
        match requested {
            ExportOption::PrivateAndPublic => {
                // For private key export, we must have the private key
                let private_key = keypair.private_key.ok_or_else(|| {
                    Error::Message("Expected private key in exported keypair".to_string())
                })?;

                // Public key is optional for private key exports
                Ok(ExportedKey::Complete {
                    private_key,
                    public_key: keypair.public_key,
                })
            }
            ExportOption::PublicKeyOnly => {
                // For public key export, we must have the public key
                let public_key = keypair.public_key.ok_or_else(|| {
                    Error::Message("Expected public key in exported keypair".to_string())
                })?;

                Ok(ExportedKey::PublicOnly(public_key))
            }
            ExportOption::ExtendedPublicKey => {
                // For extended public key export, we need both public key and chain code
                let public_key = keypair.public_key.ok_or_else(|| {
                    Error::Message("Expected public key in exported keypair".to_string())
                })?;

                let chain_code = keypair.chain_code.ok_or_else(|| {
                    Error::Message("Expected chain code in exported keypair".to_string())
                })?;

                Ok(ExportedKey::Extended {
                    public_key,
                    chain_code,
                })
            }
        }
    }

    /// Get the public key, if available
    pub fn public_key(&self) -> Option<&PublicKey> {
        match self {
            ExportedKey::Complete { public_key, .. } => public_key.as_ref(),
            ExportedKey::PublicOnly(public_key) => Some(public_key),
            ExportedKey::Extended { public_key, .. } => Some(public_key),
        }
    }

    /// Get the private key, if available
    pub fn private_key(&self) -> Option<&SecretKey> {
        match self {
            ExportedKey::Complete { private_key, .. } => Some(private_key),
            _ => None,
        }
    }

    /// Get the chain code, if available
    pub fn chain_code(&self) -> Option<&Vec<u8>> {
        match self {
            ExportedKey::Extended { chain_code, .. } => Some(chain_code),
            _ => None,
        }
    }

    /// Check if this is a complete (private + optional public) key
    pub fn is_complete(&self) -> bool {
        matches!(self, ExportedKey::Complete { .. })
    }

    /// Check if this is a public key only
    pub fn is_public_only(&self) -> bool {
        matches!(self, ExportedKey::PublicOnly(_))
    }

    /// Check if this is an extended public key
    pub fn is_extended(&self) -> bool {
        matches!(self, ExportedKey::Extended { .. })
    }
}
