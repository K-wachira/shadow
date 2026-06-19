//! Keystore: releases the per-device wrap key after the owner authenticates.

use color_eyre::Result;
use color_eyre::eyre::eyre;
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

/// A 256-bit symmetric key released by a keystore after the owner authenticates.
/// `Zeroizing` wipes it on drop.
pub type WrapKey = Zeroizing<[u8; 32]>;

/// Releases the per-device `device_wrap_key` only after a successful owner
/// authentication. The biometric *gates* this key; it never *becomes* the key
/// and never leaves the device (INV-1 / INV-2).
///
/// Phase 0 ships [`PassphraseKeystore`]. The Secure-Enclave / Touch-ID impl is
/// P0.4 and slots in behind this same trait.
pub trait Keystore {
    /// Authenticate the owner and release the device wrap key.
    fn unlock(&self) -> Result<WrapKey>;
}

/// Fallback keystore: derives the wrap key from a passphrase via Argon2id. Used
/// for development and on platforms without a hardware enclave (§5.2 honest flag).
pub struct PassphraseKeystore {
    passphrase: Zeroizing<Vec<u8>>,
    salt: [u8; 16],
}

impl PassphraseKeystore {
    pub fn new(passphrase: impl Into<Vec<u8>>, salt: [u8; 16]) -> Self {
        Self {
            passphrase: Zeroizing::new(passphrase.into()),
            salt,
        }
    }
}

impl Keystore for PassphraseKeystore {
    fn unlock(&self) -> Result<WrapKey> {
        let mut key = Zeroizing::new([0u8; 32]);
        argon2::Argon2::default()
            .hash_password_into(&self.passphrase, &self.salt, key.as_mut_slice())
            .map_err(|e| eyre!("argon2 key derivation failed: {e}"))?;
        Ok(key)
    }
}

/// Generate a fresh random 16-byte salt for the passphrase keystore. The salt
/// is not secret; it is stored alongside the sealed key material.
pub fn random_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_salt_is_not_all_zero() {
        assert_ne!(random_salt(), [0u8; 16]);
    }

    #[test]
    fn passphrase_keystore_is_deterministic() {
        let ks = PassphraseKeystore::new(b"correct horse".to_vec(), [1u8; 16]);
        let a = ks.unlock().unwrap();
        let b = ks.unlock().unwrap();
        assert_eq!(&a[..], &b[..]);
    }

    #[test]
    fn different_salt_yields_different_key() {
        let a = PassphraseKeystore::new(b"pw".to_vec(), [1u8; 16])
            .unlock()
            .unwrap();
        let b = PassphraseKeystore::new(b"pw".to_vec(), [2u8; 16])
            .unlock()
            .unwrap();
        assert_ne!(&a[..], &b[..]);
    }

    #[test]
    fn different_passphrase_yields_different_key() {
        let a = PassphraseKeystore::new(b"one".to_vec(), [0u8; 16])
            .unlock()
            .unwrap();
        let b = PassphraseKeystore::new(b"two".to_vec(), [0u8; 16])
            .unlock()
            .unwrap();
        assert_ne!(&a[..], &b[..]);
    }
}
