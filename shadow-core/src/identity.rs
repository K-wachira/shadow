//! Identity bootstrap: unlock (or mint) the Shadow ID + owner root DEK at startup.
//!
//! Phase 0 uses a passphrase-backed keystore (`SHADOW_PASSPHRASE`, with an
//! insecure dev fallback). Touch ID / Secure Enclave replaces the keystore in
//! P0.4 behind the same `Keystore` trait — see `md/NETWORK_SCOPE.md`.

use crate::config::{IdentityConfig, KeystoreBackend};
use crate::setup::ShadowPaths;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use shadow_identity::vault::{open_with, seal_with};
use shadow_identity::{
    Keystore, OfflineBackup, OwnerRootKey, PassphraseKeystore, ShadowId, ShadowKeypair, WrapKey,
    random_salt,
};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Stand-in passphrase used only when `SHADOW_PASSPHRASE` is unset. Provides no
/// real protection — it exists so dev runs work before Touch ID lands (P0.4).
const DEV_PASSPHRASE: &str = "shadow-dev-insecure";

/// An unlocked identity held in memory for the session.
pub struct UnlockedIdentity {
    pub shadow_id: ShadowId,
    pub keypair: ShadowKeypair,
    pub dek: Arc<OwnerRootKey>,
}

fn passphrase(cfg: &IdentityConfig) -> Result<Zeroizing<Vec<u8>>> {
    match std::env::var("SHADOW_PASSPHRASE") {
        Ok(p) if !p.is_empty() => Ok(Zeroizing::new(p.into_bytes())),
        _ if cfg.require_passphrase => Err(eyre!(
            "SHADOW_PASSPHRASE is unset but identity.require_passphrase = true"
        )),
        _ => {
            tracing::warn!(
                "SHADOW_PASSPHRASE unset — using an insecure dev passphrase. \
                 Set identity.require_passphrase = true to forbid this (Touch ID is P0.4)."
            );
            Ok(Zeroizing::new(DEV_PASSPHRASE.as_bytes().to_vec()))
        }
    }
}

/// Derive/release the device wrap key for the configured keystore backend.
/// This is the seam the Secure-Enclave backend (P0.4) plugs into — everything
/// downstream (sealing the Shadow ID + DEK) is backend-agnostic.
fn device_wrap_key(paths: &ShadowPaths, cfg: &IdentityConfig) -> Result<WrapKey> {
    match cfg.keystore {
        KeystoreBackend::Passphrase => {
            let salt = load_or_create_salt(&paths.identity)?;
            PassphraseKeystore::new(passphrase(cfg)?.to_vec(), salt).unlock()
        }
        KeystoreBackend::SecureEnclave => secure_enclave_wrap_key(),
    }
}

#[cfg(target_os = "macos")]
fn secure_enclave_wrap_key() -> Result<WrapKey> {
    shadow_identity::SecureEnclaveKeystore::device_default().unlock()
}

#[cfg(not(target_os = "macos"))]
fn secure_enclave_wrap_key() -> Result<WrapKey> {
    Err(eyre!(
        "identity.keystore = \"secure_enclave\" requires macOS; use \"passphrase\""
    ))
}

fn load_or_create_salt(dir: &Path) -> Result<[u8; 16]> {
    let salt_path = dir.join("wrap.salt");
    if salt_path.exists() {
        let bytes = fs::read(&salt_path)?;
        bytes
            .as_slice()
            .try_into()
            .map_err(|_| eyre!("corrupt wrap.salt"))
    } else {
        let salt = random_salt();
        fs::write(&salt_path, salt)?;
        Ok(salt)
    }
}

/// Unlock the existing identity, or mint a fresh one on first run.
pub fn unlock_or_init(paths: &ShadowPaths, cfg: &IdentityConfig) -> Result<UnlockedIdentity> {
    fs::create_dir_all(&paths.identity)?;
    let wrap = device_wrap_key(paths, cfg)?;

    let id_path = paths.identity.join("shadow_id.seal");
    let dek_path = paths.identity.join("dek.seal");

    if id_path.exists() && dek_path.exists() {
        let secret_pt = open_with(&wrap, &fs::read(&id_path)?)?;
        let secret: [u8; 32] = secret_pt
            .as_slice()
            .try_into()
            .map_err(|_| eyre!("corrupt identity key"))?;
        let keypair = ShadowKeypair::from_secret_bytes(&secret);
        let dek = OwnerRootKey::unseal(&wrap, &fs::read(&dek_path)?)?;
        Ok(UnlockedIdentity {
            shadow_id: keypair.shadow_id(),
            keypair,
            dek: Arc::new(dek),
        })
    } else {
        let keypair = ShadowKeypair::generate();
        let dek = OwnerRootKey::generate();
        let secret = Zeroizing::new(keypair.secret_bytes());
        fs::write(&id_path, seal_with(&wrap, secret.as_slice())?)?;
        fs::write(&dek_path, dek.seal(&wrap)?)?;
        tracing::info!("minted new Shadow ID");
        Ok(UnlockedIdentity {
            shadow_id: keypair.shadow_id(),
            keypair,
            dek: Arc::new(dek),
        })
    }
}

/// Export a passphrase-encrypted offline backup of this identity's root secrets
/// (P0.6). Useless without `backup_passphrase`; the network never holds it.
pub fn export_backup(identity: &UnlockedIdentity, backup_passphrase: &[u8]) -> Result<Vec<u8>> {
    let secret = Zeroizing::new(identity.keypair.secret_bytes());
    let dek = Zeroizing::new(identity.dek.expose_bytes());
    OfflineBackup::export(&secret, &dek, backup_passphrase, random_salt())
}

/// Restore an identity from a backup blob onto THIS device, re-sealing it under
/// the device keystore. Refuses to overwrite an existing identity.
pub fn restore_backup(
    paths: &ShadowPaths, blob: &[u8], backup_passphrase: &[u8], cfg: &IdentityConfig,
) -> Result<ShadowId> {
    if paths.identity.join("shadow_id.seal").exists() {
        return Err(eyre!(
            "an identity already exists at {:?}; remove it before restoring",
            paths.identity
        ));
    }
    let (secret, dek) = OfflineBackup::import(blob, backup_passphrase)?;
    restore_from_secrets(paths, &secret, &dek, cfg)?;
    Ok(ShadowKeypair::from_secret_bytes(&secret).shadow_id())
}

fn restore_from_secrets(
    paths: &ShadowPaths, shadow_secret: &[u8; 32], dek_bytes: &[u8; 32], cfg: &IdentityConfig,
) -> Result<()> {
    fs::create_dir_all(&paths.identity)?;
    let wrap = device_wrap_key(paths, cfg)?;
    fs::write(
        paths.identity.join("shadow_id.seal"),
        seal_with(&wrap, shadow_secret)?,
    )?;
    let dek = OwnerRootKey::from_bytes(*dek_bytes);
    fs::write(paths.identity.join("dek.seal"), dek.seal(&wrap)?)?;
    tracing::info!("restored Shadow ID from backup");
    Ok(())
}
