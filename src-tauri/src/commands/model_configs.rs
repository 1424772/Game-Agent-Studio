use crate::crypto::SecretStore;
use crate::models::{ModelConfig, ModelConfigPublic};
use crate::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn save_model_config(
    state: State<AppState>,
    base_url: String,
    api_key: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
) -> Result<ModelConfigPublic, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let store = SecretStore::new();
    let encrypted_api_key = store.encrypt(&api_key)?;

    let existing: Option<String> = db
        .query_row(
            "SELECT id FROM model_configs LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = existing {
        db.execute(
            "UPDATE model_configs SET base_url = ?1, encrypted_api_key = ?2, model = ?3, temperature = ?4, max_tokens = ?5, updated_at = ?6 WHERE id = ?7",
            rusqlite::params![base_url, encrypted_api_key, model, temperature, max_tokens, now, existing_id],
        )
        .map_err(|e| e.to_string())?;

        let config = ModelConfig {
            id: existing_id,
            base_url,
            encrypted_api_key,
            model,
            temperature,
            max_tokens,
            created_at: now.clone(),
            updated_at: now,
        };
        Ok(ModelConfigPublic::from_config(&config))
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO model_configs (id, base_url, encrypted_api_key, model, temperature, max_tokens, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![id, base_url, encrypted_api_key, model, temperature, max_tokens, now, now],
        )
        .map_err(|e| e.to_string())?;

        let config = ModelConfig {
            id,
            base_url,
            encrypted_api_key,
            model,
            temperature,
            max_tokens,
            created_at: now.clone(),
            updated_at: now,
        };
        Ok(ModelConfigPublic::from_config(&config))
    }
}

#[tauri::command]
pub fn get_model_config(state: State<AppState>) -> Result<Option<ModelConfigPublic>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db.query_row(
        "SELECT id, base_url, encrypted_api_key, model, temperature, max_tokens, created_at, updated_at FROM model_configs LIMIT 1",
        [],
        |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                base_url: row.get(1)?,
                encrypted_api_key: row.get(2)?,
                model: row.get(3)?,
                temperature: row.get(4)?,
                max_tokens: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    );

    match result {
        Ok(config) => Ok(Some(ModelConfigPublic::from_config(&config))),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn test_model_connection(
    base_url: String,
    api_key: String,
    model: String,
) -> Result<bool, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": "Hello, respond with just the word 'ok'."
            }
        ],
        "temperature": 0.7,
        "max_tokens": 10,
        "stream": false
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!("HTTP error: {}", status));
    }

    let json: Value = response.json().await.map_err(|e| e.to_string())?;

    json["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .ok_or_else(|| "Unexpected response format".to_string())?;

    Ok(true)
}
