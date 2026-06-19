//! The Shadow ID: the owner-controlled Ed25519 root identity.

use color_eyre::Result;
use color_eyre::eyre::eyre;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;

pub use ed25519_dalek::Signature;

/// The owner-controlled root identity: the public half of a long-lived Ed25519
/// keypair. This *is* the shadow's address — self-minted, never registered
/// (INV-8 / INV-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowId(pub VerifyingKey);

impl ShadowId {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        VerifyingKey::from_bytes(bytes)
            .map(ShadowId)
            .map_err(|e| eyre!("invalid Shadow ID bytes: {e}"))
    }

    /// Verify a signature made by this identity over `msg`.
    pub fn verify(&self, msg: &[u8], sig: &Signature) -> Result<()> {
        self.0
            .verify(msg, sig)
            .map_err(|e| eyre!("signature verification failed: {e}"))
    }
}

/// A full Shadow keypair. The inner [`SigningKey`] zeroizes on drop (the
/// `zeroize` feature of `ed25519-dalek`), so the secret never lingers in memory.
/// At rest the secret must always be sealed by the [`crate::vault::Vault`];
/// it is never written in plaintext (INV-1).
pub struct ShadowKeypair {
    signing: SigningKey,
}

impl ShadowKeypair {
    /// Mint a fresh identity from the OS CSPRNG (first run).
    pub fn generate() -> Self {
        Self {
            signing: SigningKey::generate(&mut OsRng),
        }
    }

    /// Reconstruct from the 32-byte secret seed (e.g. after unsealing the vault
    /// or importing a backup).
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            signing: SigningKey::from_bytes(bytes),
        }
    }

    /// The 32-byte secret seed. Handle only inside a sealed/backup context and
    /// zeroize the returned array after use — it is the raw private key.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    pub fn shadow_id(&self) -> ShadowId {
        ShadowId(self.signing.verifying_key())
    }

    /// Phase 0: the Shadow ID signs directly (no device-key delegation yet —
    /// that arrives in Phase 1 when signatures first cross the wire).
    pub fn sign(&self, msg: &[u8]) -> Signature {
        self.signing.sign(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sign_verify_roundtrip() {
        let kp = ShadowKeypair::generate();
        let id = kp.shadow_id();
        let msg = b"shadow speaks";
        let sig = kp.sign(msg);
        assert!(id.verify(msg, &sig).is_ok());
    }

    #[test]
    fn wrong_message_fails_verify() {
        let kp = ShadowKeypair::generate();
        let sig = kp.sign(b"a");
        assert!(kp.shadow_id().verify(b"b", &sig).is_err());
    }

    #[test]
    fn shadow_id_bytes_roundtrip() {
        let kp = ShadowKeypair::generate();
        let id = kp.shadow_id();
        let bytes = id.to_bytes();
        assert_eq!(ShadowId::from_bytes(&bytes).unwrap(), id);
    }

    #[test]
    fn secret_bytes_reconstruct_same_identity() {
        let kp = ShadowKeypair::generate();
        let sec = kp.secret_bytes();
        let kp2 = ShadowKeypair::from_secret_bytes(&sec);
        assert_eq!(kp.shadow_id(), kp2.shadow_id());
    }

    #[test]
    fn two_identities_differ() {
        let a = ShadowKeypair::generate();
        let b = ShadowKeypair::generate();
        assert_ne!(a.shadow_id(), b.shadow_id());
    }
}
