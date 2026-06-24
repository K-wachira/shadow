//! Envelope encryption for data-at-rest (INV-9).
//!
//! A single random per-shadow data-encryption key ([`OwnerRootKey`]) encrypts
//! all data at rest. The DEK is itself stored *sealed* under the device wrap key
//! released by the [`crate::keystore::Keystore`]. A new device can read synced
//! data once pairing re-wraps the *same* DEK under its own keystore — no data
//! re-encryption (see §5.4 / Phase 6).

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

use crate::keystore::WrapKey;

const NONCE_LEN: usize = 24; // XChaCha20-Poly1305

/// The per-shadow data-encryption key (DEK).
pub struct OwnerRootKey(Zeroizing<[u8; 32]>);

impl OwnerRootKey {
    /// Mint a fresh DEK from the OS CSPRNG.
    pub fn generate() -> Self {
        let mut k = [0u8; 32];
        OsRng.fill_bytes(&mut k);
        Self(Zeroizing::new(k))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Copy the raw key bytes out — for offline backup only (the result is
    /// immediately re-sealed under a backup passphrase). The returned array is
    /// secret material; zeroize it after use.
    pub fn expose_bytes(&self) -> [u8; 32] {
        *self.0
    }

    /// Seal this DEK under the device wrap key for storage at rest.
    pub fn seal(&self, wrap_key: &WrapKey) -> Result<Vec<u8>> {
        seal_with(wrap_key, self.key_bytes())
    }

    /// Recover the DEK by unsealing it with the device wrap key.
    pub fn unseal(wrap_key: &WrapKey, sealed: &[u8]) -> Result<Self> {
        let pt = open_with(wrap_key, sealed)?;
        let bytes: [u8; 32] = pt
            .as_slice()
            .try_into()
            .map_err(|_| eyre!("unsealed DEK has wrong length"))?;
        Ok(Self::from_bytes(bytes))
    }

    fn key_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Encrypts data at rest under the owner root DEK.
pub struct Vault<'a> {
    dek: &'a OwnerRootKey,
}

impl<'a> Vault<'a> {
    pub fn new(dek: &'a OwnerRootKey) -> Self {
        Self { dek }
    }

    /// Encrypt `plaintext` for storage. Output = `nonce(24) || ciphertext+tag`.
    pub fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        seal(self.dek.key_bytes(), plaintext)
    }

    /// Decrypt data produced by [`Vault::seal`].
    pub fn open(&self, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        open(self.dek.key_bytes(), sealed)
    }
}

/// Seal arbitrary bytes under a [`WrapKey`] (used for the DEK and offline backup).
pub fn seal_with(wrap_key: &WrapKey, plaintext: &[u8]) -> Result<Vec<u8>> {
    seal(&**wrap_key, plaintext)
}

/// Open bytes produced by [`seal_with`].
pub fn open_with(wrap_key: &WrapKey, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    open(&**wrap_key, sealed)
}

fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|e| eyre!("seal failed: {e}"))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn open(key: &[u8; 32], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if sealed.len() < NONCE_LEN {
        return Err(eyre!("sealed blob too short"));
    }
    let (nonce, ct) = sealed.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let pt = cipher
        .decrypt(XNonce::from_slice(nonce), ct)
        .map_err(|e| eyre!("open failed (wrong key or tampered data): {e}"))?;
    Ok(Zeroizing::new(pt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_seal_open_roundtrip() {
        let dek = OwnerRootKey::generate();
        let v = Vault::new(&dek);
        let pt = b"the person-model";
        let sealed = v.seal(pt).unwrap();
        assert_ne!(&sealed[NONCE_LEN..], &pt[..]); // ciphertext is not the plaintext
        let opened = v.open(&sealed).unwrap();
        assert_eq!(&opened[..], pt);
    }

    #[test]
    fn tampered_blob_fails_to_open() {
        let dek = OwnerRootKey::generate();
        let v = Vault::new(&dek);
        let mut sealed = v.seal(b"x").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        assert!(v.open(&sealed).is_err());
    }

    #[test]
    fn dek_seals_under_wrap_key_and_survives_roundtrip() {
        let dek = OwnerRootKey::generate();
        let wrap: WrapKey = Zeroizing::new([7u8; 32]);
        let sealed = dek.seal(&wrap).unwrap();
        let recovered = OwnerRootKey::unseal(&wrap, &sealed).unwrap();
        // The recovered DEK decrypts data the original sealed.
        let data = b"continuity log";
        let s = Vault::new(&dek).seal(data).unwrap();
        let o = Vault::new(&recovered).open(&s).unwrap();
        assert_eq!(&o[..], data);
    }

    #[test]
    fn expose_bytes_matches_from_bytes() {
        let raw = [42u8; 32];
        assert_eq!(OwnerRootKey::from_bytes(raw).expose_bytes(), raw);
    }

    #[test]
    fn wrong_wrap_key_cannot_unseal_dek() {
        let dek = OwnerRootKey::generate();
        let right: WrapKey = Zeroizing::new([1u8; 32]);
        let wrong: WrapKey = Zeroizing::new([2u8; 32]);
        let sealed = dek.seal(&right).unwrap();
        assert!(OwnerRootKey::unseal(&wrong, &sealed).is_err());
    }
}
