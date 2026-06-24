use crate::mind::mind_model::Meta;
use crate::mind::mind_model::ShadowMind;
use color_eyre::Result;
use shadow_identity::{OwnerRootKey, Vault};
use shadow_utils::utils;
use std::collections::HashMap;
use std::path::Path;

/// Read the person-model file and return its decrypted JSON text.
///
/// If the bytes don't decrypt under `dek` we assume a legacy *plaintext* mind
/// (pre-encryption) and return them as-is — so existing minds migrate
/// transparently the next time they are saved (INV-9).
pub fn read_text(mind_path: &Path, dek: &OwnerRootKey) -> Result<String> {
    let bytes = std::fs::read(mind_path)?;
    let text = match Vault::new(dek).open(&bytes) {
        Ok(plaintext) => String::from_utf8(plaintext.to_vec())?,
        Err(_) => String::from_utf8(bytes)?, // legacy plaintext migration
    };
    Ok(text)
}

/// Seal `text` under the owner root key and write it atomically (INV-9).
pub fn write_text(text: &str, mind_path: &Path, dek: &OwnerRootKey) -> Result<()> {
    if let Some(parent) = mind_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let sealed = Vault::new(dek).seal(text.as_bytes())?;
    let tmp = mind_path.with_extension("tmp");
    std::fs::write(&tmp, sealed)?;
    std::fs::rename(&tmp, mind_path)?;
    Ok(())
}

pub fn load(mind_path: &Path, dek: &OwnerRootKey) -> Result<ShadowMind> {
    if !mind_path.exists() {
        return Ok(init());
    }
    let contents = read_text(mind_path, dek)?;
    let mind: ShadowMind = json5::from_str(&contents)?;
    Ok(mind)
}

pub fn save(mind: &ShadowMind, mind_path: &Path, dek: &OwnerRootKey) -> Result<()> {
    let contents = json5::to_string(mind)?;
    write_text(&contents, mind_path, dek)
}

pub fn init() -> ShadowMind {
    ShadowMind {
        meta: Meta {
            version: 1,
            last_updated: utils::today(),
            log_range: None,
            total_logs_considered: 0,
            rewrite_trigger: String::from("init"),
        },
        surface: HashMap::new(),
        behavioural: HashMap::new(),
        mental_model: HashMap::new(),
        values: HashMap::new(),
    }
}
