//! Offline backup — interim recovery before Phase 6 social recovery (P0.6).
//!
//! Seals the secrets needed to restore a Shadow on a fresh install (the Shadow
//! ID secret seed + the owner root DEK) under an Argon2id-derived key. The blob
//! is useless without the passphrase, and the network never holds it (INV-10).

use color_eyre::Result;
use color_eyre::eyre::eyre;
use zeroize::Zeroizing;

use crate::keystore::{Keystore, PassphraseKeystore};
use crate::vault::{open_with, seal_with};

const SALT_LEN: usize = 16;
const SECRET_LEN: usize = 32;
const PAYLOAD_LEN: usize = SECRET_LEN * 2; // shadow secret || dek

/// A passphrase-encrypted offline backup of a Shadow's root secrets.
pub struct OfflineBackup;

impl OfflineBackup {
    /// Export `shadow_secret || dek` sealed under a passphrase-derived key.
    /// Output = `salt(16) || sealed`.
    pub fn export(
        shadow_secret: &[u8; SECRET_LEN], dek: &[u8; SECRET_LEN], passphrase: &[u8],
        salt: [u8; SALT_LEN],
    ) -> Result<Vec<u8>> {
        let wrap = PassphraseKeystore::new(passphrase.to_vec(), salt).unlock()?;
        let mut payload = Zeroizing::new(Vec::with_capacity(PAYLOAD_LEN));
        payload.extend_from_slice(shadow_secret);
        payload.extend_from_slice(dek);

        let mut out = Vec::with_capacity(SALT_LEN + PAYLOAD_LEN + 40);
        out.extend_from_slice(&salt);
        out.extend_from_slice(&seal_with(&wrap, &payload)?);
        Ok(out)
    }

    /// Recover `(shadow_secret, dek)` from a backup blob + passphrase.
    pub fn import(
        blob: &[u8], passphrase: &[u8],
    ) -> Result<(Zeroizing<[u8; SECRET_LEN]>, Zeroizing<[u8; SECRET_LEN]>)> {
        if blob.len() < SALT_LEN {
            return Err(eyre!("backup blob too short"));
        }
        let (salt, sealed) = blob.split_at(SALT_LEN);
        let salt: [u8; SALT_LEN] = salt.try_into().expect("checked length");

        let wrap = PassphraseKeystore::new(passphrase.to_vec(), salt).unlock()?;
        let pt = open_with(&wrap, sealed)?;
        if pt.len() != PAYLOAD_LEN {
            return Err(eyre!("backup payload has wrong length"));
        }

        let mut secret = Zeroizing::new([0u8; SECRET_LEN]);
        let mut dek = Zeroizing::new([0u8; SECRET_LEN]);
        secret.copy_from_slice(&pt[..SECRET_LEN]);
        dek.copy_from_slice(&pt[SECRET_LEN..]);
        Ok((secret, dek))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_export_import_roundtrip() {
        let secret = [3u8; SECRET_LEN];
        let dek = [9u8; SECRET_LEN];
        let blob = OfflineBackup::export(&secret, &dek, b"passphrase", [5u8; SALT_LEN]).unwrap();
        let (rs, rd) = OfflineBackup::import(&blob, b"passphrase").unwrap();
        assert_eq!(&rs[..], &secret[..]);
        assert_eq!(&rd[..], &dek[..]);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let blob = OfflineBackup::export(
            &[1u8; SECRET_LEN],
            &[2u8; SECRET_LEN],
            b"right",
            [0u8; SALT_LEN],
        )
        .unwrap();
        assert!(OfflineBackup::import(&blob, b"wrong").is_err());
    }
}
