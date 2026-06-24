//! Secure Enclave / Touch ID keystore (macOS, P0.4).
//!
//! Stores a random 32-byte device wrap key in the login Keychain behind a
//! biometric access-control (`UserPresence` = Touch ID *or* device passcode).
//! [`SecureEnclaveKeystore::unlock`] reads it, which makes macOS present the
//! Touch ID prompt; the biometric only *gates* the key — it never *becomes* the
//! key (INV-2) and the key never leaves the device (INV-1).
//!
//! NOTE: this compiles on macOS, but the biometric path can only be exercised in
//! a real GUI login session with enrolled Touch ID — not in CI/sandbox. Treat it
//! as "compiles, needs on-device verification" until run on real hardware.

use crate::keystore::{Keystore, WrapKey};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::CFTypeRef;
use core_foundation_sys::data::CFDataRef;
use rand_core::{OsRng, RngCore};
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework_sys::access_control::kSecAccessControlUserPresence;
use security_framework_sys::item::{
    kSecAttrAccessControl, kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
    kSecReturnData, kSecUseDataProtectionKeychain, kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching};
use std::ptr;
use zeroize::Zeroizing;

const SERVICE: &str = "network.shadow.device-wrap-key";
const ACCOUNT: &str = "default";

// OSStatus codes (Security framework).
const ERR_SEC_SUCCESS: i32 = 0;
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// Keystore backed by the macOS Keychain + Secure Enclave, gated by Touch ID.
pub struct SecureEnclaveKeystore {
    service: CFString,
    account: CFString,
}

impl SecureEnclaveKeystore {
    /// The single per-device wrap-key item.
    pub fn device_default() -> Self {
        Self {
            service: CFString::new(SERVICE),
            account: CFString::new(ACCOUNT),
        }
    }

    /// `{ kSecClass: GenericPassword, kSecAttrService, kSecAttrAccount }`.
    fn base_pairs(&self) -> Vec<(CFType, CFType)> {
        unsafe {
            vec![
                // Route to the data-protection keychain, where the
                // keychain-access-group entitlement + biometric ACL apply.
                (
                    CFString::wrap_under_get_rule(kSecUseDataProtectionKeychain).as_CFType(),
                    CFBoolean::true_value().as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecClass).as_CFType(),
                    CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrService).as_CFType(),
                    self.service.as_CFType(),
                ),
                (
                    CFString::wrap_under_get_rule(kSecAttrAccount).as_CFType(),
                    self.account.as_CFType(),
                ),
            ]
        }
    }

    /// Read the wrap key (triggers the Touch ID prompt). `Ok(None)` if absent.
    fn read(&self) -> Result<Option<WrapKey>> {
        let mut pairs = self.base_pairs();
        unsafe {
            pairs.push((
                CFString::wrap_under_get_rule(kSecReturnData).as_CFType(),
                CFBoolean::true_value().as_CFType(),
            ));
        }
        let query = CFDictionary::from_CFType_pairs(&pairs);

        let mut result: CFTypeRef = ptr::null();
        let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };
        match status {
            ERR_SEC_SUCCESS => {
                if result.is_null() {
                    return Err(eyre!("keychain returned success but no data"));
                }
                let data = unsafe { CFData::wrap_under_create_rule(result as CFDataRef) };
                let arr: [u8; 32] = data
                    .bytes()
                    .try_into()
                    .map_err(|_| eyre!("device wrap key has wrong length"))?;
                Ok(Some(Zeroizing::new(arr)))
            }
            ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            other => Err(eyre!("keychain read failed (OSStatus {other})")),
        }
    }

    /// Mint a fresh wrap key and store it behind biometric access control.
    fn create(&self) -> Result<WrapKey> {
        let mut key = Zeroizing::new([0u8; 32]);
        OsRng.fill_bytes(key.as_mut_slice());

        let access = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            kSecAccessControlUserPresence,
        )
        .map_err(|e| eyre!("SecAccessControlCreateWithFlags failed: {e}"))?;

        let data = CFData::from_buffer(key.as_slice());
        let mut pairs = self.base_pairs();
        unsafe {
            pairs.push((
                CFString::wrap_under_get_rule(kSecValueData).as_CFType(),
                data.as_CFType(),
            ));
            pairs.push((
                CFString::wrap_under_get_rule(kSecAttrAccessControl).as_CFType(),
                access.as_CFType(),
            ));
        }
        let attrs = CFDictionary::from_CFType_pairs(&pairs);

        let status = unsafe { SecItemAdd(attrs.as_concrete_TypeRef(), ptr::null_mut()) };
        if status != ERR_SEC_SUCCESS {
            return Err(eyre!("keychain add failed (OSStatus {status})"));
        }
        Ok(key)
    }
}

impl Keystore for SecureEnclaveKeystore {
    fn unlock(&self) -> Result<WrapKey> {
        match self.read()? {
            Some(key) => Ok(key),
            None => self.create(),
        }
    }
}
