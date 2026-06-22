use crate::models::{MemoryLayer, MemoryScope, ProjectMemory, UserPreference};
use crate::AppState;
use tauri::State;

fn parse_memory_layer(s: &str) -> MemoryLayer {
    match s.to_lowercase().as_str() {
        "l4" => MemoryLayer::L4,
        "l3" => MemoryLayer::L3,
        "l2" => MemoryLayer::L2,
        _ => MemoryLayer::L1,
    }
}

fn parse_memory_scope(s: &str) -> MemoryScope {
    match s.to_lowercase().as_str() {
        "session" => MemoryScope::Session,
        "global" => MemoryScope::Global,
        _ => MemoryScope::Project,
    }
}

fn default_layer() -> String {
    "L1".to_string()
}

fn default_scope() -> String {
    "project".to_string()
}

fn default_confidence() -> f64 {
    1.0
}

fn default_version() -> i32 {
    1
}

fn default_source() -> Option<String> {
    None
}

fn default_provenance() -> Option<String> {
    None
}

#[tauri::command]
pub fn get_project_memory(
    state: State<AppState>,
    project_id: String,
    memory_type: Option<String>,
) -> Result<Vec<ProjectMemory>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let select_sql = "SELECT id, project_id, memory_type, key, value, layer, scope, source, confidence, version, provenance, created_at, updated_at FROM project_memory";

    let entries = match memory_type {
        Some(mt) => {
            let sql = format!("{} WHERE project_id = ?1 AND memory_type = ?2 ORDER BY key ASC", select_sql);
            let mut stmt = db
                .prepare(&sql)
                .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            let rows = stmt
                .query_map(rusqlite::params![project_id, mt], |row| {
                    read_memory_row(row)
                })
                .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            rows.collect::<Result<Vec<ProjectMemory>, _>>()
                .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        }
        None => {
            let sql = format!("{} WHERE project_id = ?1 ORDER BY memory_type, key ASC", select_sql);
            let mut stmt = db
                .prepare(&sql)
                .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            let rows = stmt
                .query_map(rusqlite::params![project_id], |row| {
                    read_memory_row(row)
                })
                .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            rows.collect::<Result<Vec<ProjectMemory>, _>>()
                .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        }
    };

    Ok(entries)
}

fn read_memory_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectMemory> {
    let layer_str: String = row.get::<_, String>(5).unwrap_or_else(|_| "L1".to_string());
    let scope_str: String = row.get::<_, String>(6).unwrap_or_else(|_| "project".to_string());
    let confidence_val: f64 = row.get::<_, f64>(8).unwrap_or(1.0);
    let version_val: i32 = row.get::<_, i32>(9).unwrap_or(1);
    Ok(ProjectMemory {
        id: row.get(0)?,
        project_id: row.get(1)?,
        memory_type: row.get(2)?,
        key: row.get(3)?,
        value: row.get(4)?,
        layer: parse_memory_layer(&layer_str),
        scope: parse_memory_scope(&scope_str),
        source: row.get(7).ok(),
        confidence: confidence_val,
        version: version_val,
        provenance: row.get(10).ok(),
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_project_memory(
    state: State<AppState>,
    project_id: String,
    memory_type: String,
    key: String,
    value: String,
    layer: Option<String>,
    scope: Option<String>,
    source: Option<String>,
    confidence: Option<f64>,
    version: Option<i32>,
    provenance: Option<String>,
) -> Result<ProjectMemory, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    let layer_val = layer.unwrap_or_else(default_layer);
    let scope_val = scope.unwrap_or_else(default_scope);
    let source_val = source.or_else(default_source);
    let confidence_val = confidence.unwrap_or_else(default_confidence);
    let version_val = version.unwrap_or_else(default_version);
    let provenance_val = provenance.or_else(default_provenance);

    let parsed_layer = parse_memory_layer(&layer_val);
    let parsed_scope = parse_memory_scope(&scope_val);

    let existing: Option<String> = db
        .query_row(
            "SELECT id FROM project_memory WHERE project_id = ?1 AND memory_type = ?2 AND key = ?3",
            rusqlite::params![project_id, memory_type, key],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = existing {
        db.execute(
            "UPDATE project_memory SET value = ?1, layer = ?2, scope = ?3, source = ?4, confidence = ?5, version = ?6, provenance = ?7, updated_at = ?8 WHERE id = ?9",
            rusqlite::params![
                value,
                layer_val,
                scope_val,
                source_val,
                confidence_val,
                version_val,
                provenance_val,
                now,
                existing_id,
            ],
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        Ok(ProjectMemory {
            id: existing_id,
            project_id,
            memory_type,
            key,
            value,
            layer: parsed_layer,
            scope: parsed_scope,
            source: source_val,
            confidence: confidence_val,
            version: version_val,
            provenance: provenance_val,
            created_at: now.clone(),
            updated_at: now,
        })
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO project_memory (id, project_id, memory_type, key, value, layer, scope, source, confidence, version, provenance, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                id,
                project_id,
                memory_type,
                key,
                value,
                layer_val,
                scope_val,
                source_val,
                confidence_val,
                version_val,
                provenance_val,
                now,
                now,
            ],
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        Ok(ProjectMemory {
            id,
            project_id,
            memory_type,
            key,
            value,
            layer: parsed_layer,
            scope: parsed_scope,
            source: source_val,
            confidence: confidence_val,
            version: version_val,
            provenance: provenance_val,
            created_at: now.clone(),
            updated_at: now,
        })
    }
}

#[tauri::command]
pub fn get_user_preferences(state: State<AppState>) -> Result<Vec<UserPreference>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db
        .prepare("SELECT id, preference_key, preference_value, confidence, evidence, updated_at FROM user_preferences ORDER BY preference_key ASC")
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let prefs = stmt
        .query_map([], |row| {
            Ok(UserPreference {
                id: row.get(0)?,
                preference_key: row.get(1)?,
                preference_value: row.get(2)?,
                confidence: row.get(3)?,
                evidence: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .collect::<Result<Vec<UserPreference>, _>>()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(prefs)
}

#[tauri::command]
pub fn update_user_preferences(
    state: State<AppState>,
    preference_key: String,
    preference_value: String,
    confidence: f64,
    evidence: String,
) -> Result<UserPreference, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    let existing: Option<String> = db
        .query_row(
            "SELECT id FROM user_preferences WHERE preference_key = ?1",
            rusqlite::params![preference_key],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = existing {
        db.execute(
            "UPDATE user_preferences SET preference_value = ?1, confidence = ?2, evidence = ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![preference_value, confidence, evidence, now, existing_id],
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        Ok(UserPreference {
            id: existing_id,
            preference_key,
            preference_value,
            confidence,
            evidence: Some(evidence),
            updated_at: now,
        })
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO user_preferences (id, preference_key, preference_value, confidence, evidence, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, preference_key, preference_value, confidence, evidence, now],
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        Ok(UserPreference {
            id,
            preference_key,
            preference_value,
            confidence,
            evidence: Some(evidence),
            updated_at: now,
        })
    }
}
