use crate::commands::security;
use crate::crypto::{create_secret_store, decrypt_saved_api_key};
use crate::models::{ModelConfig, ModelConfigPublic};
use crate::AppState;
use serde_json::Value;
use tauri::State;

fn resolve_api_key_for_test(
    db: &rusqlite::Connection,
    provided_key: Option<&str>,
) -> Result<String, String> {
    match provided_key {
        Some(key) if !key.is_empty() => Ok(key.to_string()),
        _ => {
            let stored: String = db
                .query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0))
                .map_err(|_| {
                    "API key is required for test connection: no saved key and no key provided"
                        .to_string()
                })?;
            decrypt_saved_api_key(&stored)
        }
    }
}

fn save_model_config_impl(
    db: &rusqlite::Connection,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<ModelConfigPublic, String> {
    security::validate_base_url(base_url)?;
    security::validate_max_tokens(max_tokens)?;

    let now = chrono::Utc::now().to_rfc3339();

    let encrypted_api_key = match api_key {
        Some(key) if !key.trim().is_empty() => {
            let store = create_secret_store();
            store.encrypt(key).map_err(|e| crate::models::sanitize_error(e))?
        }
        _ => {
            let existing: Option<String> = db
                .query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0))
                .ok();
            match existing {
                Some(stored) if !stored.is_empty() => stored,
                _ => return Err("api_key is required for new model config".to_string()),
            }
        }
    };

    let existing_id: Option<String> = db
        .query_row("SELECT id FROM model_configs LIMIT 1", [], |r| r.get(0))
        .ok();

    if let Some(eid) = existing_id {
        db.execute(
            "UPDATE model_configs SET base_url=?1, encrypted_api_key=?2, model=?3, temperature=?4, max_tokens=?5, updated_at=?6 WHERE id=?7",
            rusqlite::params![base_url, encrypted_api_key, model, temperature, max_tokens, now, eid],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        let config = ModelConfig { id: eid, base_url: base_url.to_string(), encrypted_api_key, model: model.to_string(), temperature, max_tokens, created_at: now.clone(), updated_at: now };
        Ok(ModelConfigPublic::from_config(&config))
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO model_configs (id, base_url, encrypted_api_key, model, temperature, max_tokens, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![id, base_url, encrypted_api_key, model, temperature, max_tokens, now, now],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        let config = ModelConfig { id, base_url: base_url.to_string(), encrypted_api_key, model: model.to_string(), temperature, max_tokens, created_at: now.clone(), updated_at: now };
        Ok(ModelConfigPublic::from_config(&config))
    }
}

#[tauri::command]
pub fn save_model_config(
    state: State<AppState>,
    base_url: String, api_key: Option<String>, model: String,
    temperature: f64, max_tokens: u32,
) -> Result<ModelConfigPublic, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    save_model_config_impl(&db, &base_url, api_key.as_deref(), &model, temperature, max_tokens)
}

#[tauri::command]
pub fn get_model_config(state: State<AppState>) -> Result<Option<ModelConfigPublic>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let result = db.query_row(
        "SELECT id, base_url, encrypted_api_key, model, temperature, max_tokens, created_at, updated_at FROM model_configs LIMIT 1",
        [], |r| Ok(ModelConfig {
            id: r.get(0)?, base_url: r.get(1)?, encrypted_api_key: r.get(2)?,
            model: r.get(3)?, temperature: r.get(4)?, max_tokens: r.get(5)?,
            created_at: r.get(6)?, updated_at: r.get(7)?,
        }),
    );
    match result {
        Ok(config) => Ok(Some(ModelConfigPublic::from_config(&config))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(crate::models::sanitize_error(e.to_string())),
    }
}

#[tauri::command]
pub async fn test_model_connection(
    state: State<'_, AppState>,
    base_url: String,
    api_key: Option<String>,
    model: String,
) -> Result<bool, String> {
    security::validate_base_url(&base_url)?;

    let api_key = {
        let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        resolve_api_key_for_test(&db, api_key.as_deref())?
    };

    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model, "messages": [{"role":"user","content":"respond 'ok'"}],
        "temperature": 0.7, "max_tokens": 10, "stream": false
    });
    let client = security::build_reqwest_client();
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json").json(&body).send().await
        .map_err(|e| crate::models::sanitize_error(format!("Connection failed: {}", e)))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    let json: Value = response.json().await.map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    json["choices"][0]["message"]["content"].as_str()
        .ok_or_else(|| "Unexpected response format".to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::LocalEncryptedSecretStore;
    use crate::crypto::SecretStore;

    fn setup_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrations::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn save_config_no_key_new_config_errors() {
        let db = setup_db();
        let r = save_model_config_impl(&db, "https://api.test.com", None, "gpt", 0.7, 4096);
        assert!(r.is_err());
    }

    #[test]
    fn save_config_empty_key_new_config_errors() {
        let db = setup_db();
        let r = save_model_config_impl(&db, "https://api.test.com", Some(""), "gpt", 0.7, 4096);
        assert!(r.is_err());
    }

    #[test]
    fn save_config_with_key_stores_marker() {
        let db = setup_db();
        let r = save_model_config_impl(&db, "https://api.test.com", Some("sk-test"), "gpt", 0.7, 4096);
        assert!(r.is_ok());
        let stored: String = db.query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap();
        assert!(!stored.is_empty());
        // Must be either "keychain_stored" marker or legacy ciphertext — NOT visible "sk-test"
        assert!(!stored.contains("sk-test"));
    }

    #[test]
    fn save_config_null_key_preserves_existing() {
        let db = setup_db();
        // First save with a key
        save_model_config_impl(&db, "https://api.test.com", Some("sk-first"), "gpt", 0.7, 4096).unwrap();
        let first: String = db.query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap();
        // Save again with null key
        let r = save_model_config_impl(&db, "https://api.test.com", None, "gpt2", 0.5, 4096);
        assert!(r.is_ok());
        let second: String = db.query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(second, first, "stored key reference must be preserved");
    }

    #[test]
    fn get_model_config_never_returns_full_key() {
        let db = setup_db();
        save_model_config_impl(&db, "https://api.test.com", Some("sk-secret-123"), "gpt", 0.7, 4096).unwrap();
        let stored: String = db.query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap();
        let has = ModelConfig { id: "x".into(), base_url: "".into(), encrypted_api_key: stored, model: "".into(), temperature: 0.0, max_tokens: 0, created_at: "".into(), updated_at: "".into() };
        let public = ModelConfigPublic::from_config(&has);
        assert!(public.has_api_key || !public.masked_api_key.is_empty());
        assert!(!public.masked_api_key.contains("sk-secret"));
    }

    #[test]
    fn decrypt_saved_key_validates_empty() {
        assert!(decrypt_saved_api_key("").is_err());
    }

    #[test]
    fn decrypt_saved_key_legacy_local() {
        let store = LocalEncryptedSecretStore::new();
        let ct = store.encrypt("sk-legacy-test").unwrap();
        assert_eq!(decrypt_saved_api_key(&ct).unwrap(), "sk-legacy-test");
    }

    #[test]
    fn decrypt_saved_key_invalid_fails() {
        assert!(decrypt_saved_api_key("bad_data").is_err());
    }
}
