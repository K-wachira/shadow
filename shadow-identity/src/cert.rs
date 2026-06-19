//! Device delegation certificate (§5.3).
//!
//! Defined now; issuance and chain verification are exercised in Phase 6, when
//! N device keys act for one Shadow ID. NOTE: the *signed* bytes must use the
//! canonical encoding from `shadow-crypto` (Phase 1) — these structs are the
//! data shape only, not yet the wire/signing format (INV-6).

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::keys::ShadowId;

/// Capabilities a delegation grants a device key. Expanded in Phase 6.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    pub can_sign_assertions: bool,
    pub can_gossip: bool,
}

/// Authorizes a per-device key to act for a Shadow ID. The `sig` is produced by
/// the Shadow ID over the (future, canonical) certificate bytes.
#[derive(Debug, Clone)]
pub struct DelegationCert {
    pub shadow_id: ShadowId,
    pub device_pub: VerifyingKey,
    pub capabilities: Capabilities,
    /// Expiry, unix seconds.
    pub not_after: u64,
    pub cert_id: [u8; 16],
    pub sig: Signature,
}
