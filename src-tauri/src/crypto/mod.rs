use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 12;
const KEYRING_SERVICE: &str = "game-agent-studio";
const KEYRING_USERNAME: &str = "llm-api-key";

pub trait SecretStore {
    fn encrypt(&self, plaintext: &str) -> Result<String, String>;
    fn decrypt(&self, encoded: &str) -> Result<String, String>;
}

// ════════════════════════════════════════════════════════════
// LocalEncryptedSecretStore — V1 fallback (hostname-derived key)
// ════════════════════════════════════════════════════════════

pub struct LocalEncryptedSecretStore {
    cipher: Aes256Gcm,
}

impl LocalEncryptedSecretStore {
    pub fn new() -> Self {
        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let salt = b"game-agent-studio-v1-salt-2024";
        let mut hasher = Sha256::new();
        hasher.update(hostname.as_bytes());
        hasher.update(salt);
        let hash = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        let cipher = Aes256Gcm::new_from_slice(&key).expect("Valid key length");
        LocalEncryptedSecretStore { cipher }
    }
}

impl SecretStore for LocalEncryptedSecretStore {
    fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self.cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&combined))
    }

    fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let data = BASE64.decode(encoded)
            .map_err(|e| format!("Decode failed: {}", e))?;
        if data.len() < NONCE_LEN + 16 {
            return Err("Invalid encrypted data".to_string());
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;
        String::from_utf8(plaintext).map_err(|e| format!("UTF-8 error: {}", e))
    }
}

// ════════════════════════════════════════════════════════════
// KeychainSecretStore — OS-native credential storage
// ════════════════════════════════════════════════════════════

pub struct KeychainSecretStore {
    entry: keyring::Entry,
}

impl KeychainSecretStore {
    pub fn new() -> Self {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
            .expect("Failed to create keyring entry");
        KeychainSecretStore { entry }
    }

    /// Try to create; returns Err if OS keychain is unavailable.
    pub fn try_new() -> Result<Self, String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME)
            .map_err(|e| format!("Keychain unavailable: {}", e))?;
        Ok(KeychainSecretStore { entry })
    }

    /// Delete the stored credential from the OS keychain.
    pub fn delete(&self) -> Result<(), String> {
        self.entry.delete_credential()
            .map_err(|e| format!("Failed to delete keychain credential: {}", e))
    }
}

impl SecretStore for KeychainSecretStore {
    fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        self.entry.set_password(plaintext)
            .map_err(|e| crate::models::sanitize_error(format!("Keychain write error: {}", e)))?;
        Ok("keychain_stored".to_string())
    }

    fn decrypt(&self, _encoded: &str) -> Result<String, String> {
        self.entry.get_password()
            .map_err(|e| crate::models::sanitize_error(format!("Keychain read error: {}", e)))
    }
}

// ════════════════════════════════════════════════════════════
// Factory — platform-aware
// ════════════════════════════════════════════════════════════

pub fn create_secret_store() -> Box<dyn SecretStore + Send + Sync> {
    match KeychainSecretStore::try_new() {
        Ok(store) => Box::new(store),
        Err(_e) => Box::new(LocalEncryptedSecretStore::new()),
    }
}

pub fn is_keychain_available() -> bool {
    KeychainSecretStore::try_new().is_ok()
}

/// Unified API key decryption from stored value.
/// - "keychain_stored" → reads from OS keychain
/// - legacy ciphertext → decrypts via LocalEncryptedSecretStore
/// - empty string → returns sanitized error
pub fn decrypt_saved_api_key(stored_value: &str) -> Result<String, String> {
    let trimmed = stored_value.trim();
    if trimmed.is_empty() {
        return Err(crate::models::sanitize_error(
            "No saved API key found. Please configure a model API key.".into(),
        ));
    }
    if trimmed == "keychain_stored" {
        return KeychainSecretStore::try_new()
            .and_then(|k| k.decrypt(""))
            .map_err(|e| crate::models::sanitize_error(e));
    }
    // Legacy: LocalEncryptedSecretStore ciphertext
    LocalEncryptedSecretStore::new()
        .decrypt(trimmed)
        .map_err(|e| crate::models::sanitize_error(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_store_encrypt_decrypt_roundtrip() {
        let store = LocalEncryptedSecretStore::new();
        let plain = "sk-test-api-key-12345";
        let encrypted = store.encrypt(plain).unwrap();
        assert_ne!(encrypted, plain);
        let decrypted = store.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn local_store_rejects_invalid_data() {
        let store = LocalEncryptedSecretStore::new();
        assert!(store.decrypt("garbage").is_err());
        assert!(store.decrypt("").is_err());
    }

    #[test]
    fn keychain_store_try_new_does_not_panic() {
        // Should succeed or return Err gracefully; must not panic
        let _ = KeychainSecretStore::try_new();
    }

    #[test]
    fn keychain_store_encrypt_result_is_deterministic() {
        match KeychainSecretStore::try_new() {
            Ok(store) => {
                // On platforms where keychain is available, encrypt returns the marker
                let result = store.encrypt("test");
                // May fail if keychain is locked, but must not panic
                if let Ok(val) = result {
                    assert_eq!(val, "keychain_stored");
                }
            }
            Err(_) => { /* keychain unavailable — expected on headless CI */ }
        }
    }

    #[test]
    fn factory_does_not_panic() {
        let _store = create_secret_store();
    }
}
