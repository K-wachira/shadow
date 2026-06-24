//! shadow-identity — self-sovereign identity for a Shadow.
//!
//! Phase 0 scope (see `md/NETWORK_SCOPE.md`):
//!   * Ed25519 Shadow ID keypair — the owner-controlled root identity / address.
//!   * Envelope encryption for data-at-rest: `device_wrap_key` -> `owner_root_dek`.
//!   * A [`Keystore`] trait with a passphrase fallback (the Secure-Enclave impl is P0.4).
//!   * An offline backup export/import for interim recovery (P0.6).
//!
//! Invariants enforced here: INV-1 (identity key never at rest in plaintext),
//! INV-9 (data encrypted at rest). Canonical signing (INV-6) arrives with
//! `shadow-crypto` in Phase 1; Phase 0 signs over raw bytes as a smoke test.

pub mod backup;
pub mod cert;
#[cfg(target_os = "macos")]
pub mod enclave;
pub mod keys;
pub mod keystore;
pub mod vault;

pub use backup::OfflineBackup;
pub use cert::{Capabilities, DelegationCert};
#[cfg(target_os = "macos")]
pub use enclave::SecureEnclaveKeystore;
pub use keys::{ShadowId, ShadowKeypair, Signature};
pub use keystore::{Keystore, PassphraseKeystore, WrapKey, random_salt};
pub use vault::{OwnerRootKey, Vault};
