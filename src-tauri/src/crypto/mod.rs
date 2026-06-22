use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 12;

pub trait SecretStore {
    fn encrypt(&self, plaintext: &str) -> Result<String, String>;
    fn decrypt(&self, encoded: &str) -> Result<String, String>;
}

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

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&combined))
    }

    fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let data = BASE64
            .decode(encoded)
            .map_err(|e| format!("Decode failed: {}", e))?;

        if data.len() < NONCE_LEN + 16 {
            return Err("Invalid encrypted data".to_string());
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| format!("UTF-8 error: {}", e))
    }
}

pub struct KeychainSecretStore;

impl KeychainSecretStore {
    pub fn new() -> Self {
        KeychainSecretStore
    }
}

impl SecretStore for KeychainSecretStore {
    fn encrypt(&self, _plaintext: &str) -> Result<String, String> {
        Err("KeychainSecretStore: OS keychain not yet implemented. Use LocalEncryptedSecretStore.".to_string())
    }

    fn decrypt(&self, _encoded: &str) -> Result<String, String> {
        Err("KeychainSecretStore: OS keychain not yet implemented. Use LocalEncryptedSecretStore.".to_string())
    }
}

pub fn create_secret_store() -> impl SecretStore {
    LocalEncryptedSecretStore::new()
}
