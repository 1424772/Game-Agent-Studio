use crate::models::{MemoryLayer, MemoryScope, MemoryVersion, ProjectMemory, UserPreference};
use crate::AppState;
use tauri::State;

// ── Layer semantics ─────────────────────────────────────────

pub struct LayerDefinition {
    pub layer: MemoryLayer,
    pub name: &'static str,
    pub description: &'static str,
    pub allowed_scopes: &'static [MemoryScope],
}

pub const LAYER_DEFINITIONS: &[LayerDefinition] = &[
    LayerDefinition {
        layer: MemoryLayer::L1,
        name: "Session Memory",
        description: "Current task context, conversation goals, temporary constraints, agent execution state.",
        allowed_scopes: &[MemoryScope::Session],
    },
    LayerDefinition {
        layer: MemoryLayer::L2,
        name: "Project Memory",
        description: "World setting, characters, plot, rules, cards/items/levels, art style, rejected ideas, export records.",
        allowed_scopes: &[MemoryScope::Project],
    },
    LayerDefinition {
        layer: MemoryLayer::L3,
        name: "User Preference Memory",
        description: "Preferred game types, platforms, art styles, models, narrative length, plot pacing, rule complexity.",
        allowed_scopes: &[MemoryScope::Global],
    },
    LayerDefinition {
        layer: MemoryLayer::L4,
        name: "System Evolution Memory",
        description: "Agent workflow success rate, prompt template effectiveness, user-accepted/rejected system improvements.",
        allowed_scopes: &[MemoryScope::Project, MemoryScope::Global],
    },
];

pub fn get_layer_definition(layer: &MemoryLayer) -> Option<&'static LayerDefinition> {
    LAYER_DEFINITIONS.iter().find(|d| d.layer == *layer)
}

pub fn validate_layer_scope(layer: &MemoryLayer, scope: &MemoryScope) -> Result<(), String> {
    if let Some(def) = get_layer_definition(layer) {
        if def.allowed_scopes.contains(scope) {
            return Ok(());
        }
    }
    Err(format!("Layer {:?} does not allow scope {:?}", layer, scope))
}

// ── Parsers ─────────────────────────────────────────────────

fn parse_memory_layer(s: &str) -> MemoryLayer {
    match s.to_lowercase().as_str() {
        "l4" => MemoryLayer::L4, "l3" => MemoryLayer::L3,
        "l2" => MemoryLayer::L2, _ => MemoryLayer::L1,
    }
}

fn parse_memory_scope(s: &str) -> MemoryScope {
    match s.to_lowercase().as_str() {
        "session" => MemoryScope::Session,
        "global" => MemoryScope::Global,
        _ => MemoryScope::Project,
    }
}

// ── Validation ──────────────────────────────────────────────

const ALLOWED_MEMORY_TYPES: &[&str] = &[
    "world_setting", "character", "plot", "rule", "card", "item",
    "level", "art_style", "rejected_idea", "export_record",
    "qa_review", "system_internal",
];

fn validate_memory_type(mt: &str) -> Result<(), String> {
    if ALLOWED_MEMORY_TYPES.contains(&mt) { Ok(()) }
    else { Err(format!("Invalid memory_type '{}'", mt)) }
}

fn validate_layer(l: &str) -> Result<MemoryLayer, String> {
    match l.to_lowercase().as_str() {
        "l1" => Ok(MemoryLayer::L1), "l2" => Ok(MemoryLayer::L2),
        "l3" => Ok(MemoryLayer::L3), "l4" => Ok(MemoryLayer::L4),
        _ => Err(format!("Invalid layer '{}'", l)),
    }
}

fn validate_scope(s: &str) -> Result<MemoryScope, String> {
    match s.to_lowercase().as_str() {
        "project" => Ok(MemoryScope::Project),
        "session" => Ok(MemoryScope::Session),
        "global" => Ok(MemoryScope::Global),
        _ => Err(format!("Invalid scope '{}'", s)),
    }
}

fn validate_confidence(c: f64) -> Result<f64, String> {
    if (0.0..=1.0).contains(&c) { Ok(c) }
    else { Err(format!("Invalid confidence {}", c)) }
}

fn validate_version(v: i32) -> Result<i32, String> {
    if v >= 1 { Ok(v) } else { Err(format!("Invalid version {}", v)) }
}

// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn get_project_memory(
    state: State<AppState>, project_id: String, memory_type: Option<String>,
) -> Result<Vec<ProjectMemory>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let base = "SELECT id, project_id, memory_type, key, value, layer, scope, source, confidence, version, provenance, created_at, updated_at FROM project_memory";
    let entries = match memory_type {
        Some(mt) => {
            let mut stmt = db.prepare(&format!("{} WHERE project_id=?1 AND memory_type=?2 ORDER BY key", base)).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            let rows = stmt.query_map(rusqlite::params![project_id, mt], |r| read_memory_row(r)).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            rows.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))?
        }
        None => {
            let mut stmt = db.prepare(&format!("{} WHERE project_id=?1 ORDER BY memory_type, key", base)).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            let rows = stmt.query_map(rusqlite::params![project_id], |r| read_memory_row(r)).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
            rows.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))?
        }
    };
    Ok(entries)
}

fn read_memory_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectMemory> {
    let layer_str: String = row.get::<_,String>(5).unwrap_or_else(|_| "L1".into());
    let scope_str: String = row.get::<_,String>(6).unwrap_or_else(|_| "project".into());
    Ok(ProjectMemory {
        id: row.get(0)?, project_id: row.get(1)?, memory_type: row.get(2)?, key: row.get(3)?,
        value: row.get(4)?, layer: parse_memory_layer(&layer_str), scope: parse_memory_scope(&scope_str),
        source: row.get(7).ok(), confidence: row.get::<_,f64>(8).unwrap_or(1.0),
        version: row.get::<_,i32>(9).unwrap_or(1), provenance: row.get(10).ok(),
        created_at: row.get(11)?, updated_at: row.get(12)?,
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_project_memory(
    state: State<AppState>, project_id: String, memory_type: String, key: String, value: String,
    layer: Option<String>, scope: Option<String>, source: Option<String>,
    confidence: Option<f64>, version: Option<i32>, provenance: Option<String>,
) -> Result<ProjectMemory, String> {
    validate_memory_type(&memory_type)?;
    let layer_str = layer.unwrap_or_else(|| "L1".into());
    let scope_str = scope.unwrap_or_else(|| "project".into());
    let confidence_val = confidence.unwrap_or(1.0);
    let version_val = version.unwrap_or(1);
    let parsed_layer = validate_layer(&layer_str)?;
    let parsed_scope = validate_scope(&scope_str)?;
    validate_confidence(confidence_val)?;
    validate_version(version_val)?;
    validate_layer_scope(&parsed_layer, &parsed_scope)?;

    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    let existing: Option<(String, String, String)> = db.query_row(
        "SELECT id, value, created_at FROM project_memory WHERE project_id=?1 AND memory_type=?2 AND key=?3",
        rusqlite::params![project_id, memory_type, key],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).ok();

    if let Some((existing_id, old_value, original_created_at)) = existing {
        let tx = db.transaction().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        let ver_id = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO memory_versions (id, memory_id, project_id, memory_type, key, old_value, new_value, source, provenance, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![ver_id, existing_id, project_id, memory_type, key, old_value, value, source, provenance, now],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        tx.execute(
            "UPDATE project_memory SET value=?1, layer=?2, scope=?3, source=?4, confidence=?5, version=?6, provenance=?7, updated_at=?8 WHERE id=?9",
            rusqlite::params![value, layer_str, scope_str, source, confidence_val, version_val, provenance, now, existing_id],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        tx.commit().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

        Ok(ProjectMemory {
            id: existing_id, project_id, memory_type, key, value,
            layer: parsed_layer, scope: parsed_scope, source, confidence: confidence_val,
            version: version_val, provenance, created_at: original_created_at, updated_at: now,
        })
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO project_memory (id,project_id,memory_type,key,value,layer,scope,source,confidence,version,provenance,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            rusqlite::params![id, project_id, memory_type, key, value, layer_str, scope_str, source, confidence_val, version_val, provenance, now, now],
        ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        Ok(ProjectMemory {
            id, project_id, memory_type, key, value,
            layer: parsed_layer, scope: parsed_scope, source, confidence: confidence_val,
            version: version_val, provenance, created_at: now.clone(), updated_at: now,
        })
    }
}

#[tauri::command]
pub fn get_memory_versions(
    state: State<AppState>, memory_id: String,
) -> Result<Vec<MemoryVersion>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare(
        "SELECT id, memory_id, project_id, memory_type, key, old_value, new_value, source, provenance, created_at FROM memory_versions WHERE memory_id=?1 ORDER BY created_at DESC"
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let versions = stmt.query_map(rusqlite::params![memory_id], |r| {
        Ok(MemoryVersion {
            id: r.get(0)?, memory_id: r.get(1)?, project_id: r.get(2)?,
            memory_type: r.get(3)?, key: r.get(4)?, old_value: r.get(5)?,
            new_value: r.get(6)?, source: r.get(7).ok(), provenance: r.get(8).ok(),
            created_at: r.get(9)?,
        })
    }).map_err(|e| crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(versions)
}

#[tauri::command]
pub fn get_user_preferences(state: State<AppState>) -> Result<Vec<UserPreference>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id, preference_key, preference_value, confidence, evidence, updated_at FROM user_preferences ORDER BY preference_key").map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let prefs = stmt.query_map([], |r| Ok(UserPreference { id: r.get(0)?, preference_key: r.get(1)?, preference_value: r.get(2)?, confidence: r.get(3)?, evidence: r.get(4)?, updated_at: r.get(5)? })).map_err(|e| crate::models::sanitize_error(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(prefs)
}

#[tauri::command]
pub fn update_user_preferences(
    state: State<AppState>, preference_key: String, preference_value: String, confidence: f64, evidence: String,
) -> Result<UserPreference, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();
    let existing: Option<String> = db.query_row("SELECT id FROM user_preferences WHERE preference_key=?1", rusqlite::params![preference_key], |r| r.get(0)).ok();
    if let Some(eid) = existing {
        db.execute("UPDATE user_preferences SET preference_value=?1,confidence=?2,evidence=?3,updated_at=?4 WHERE id=?5", rusqlite::params![preference_value,confidence,evidence,now,eid]).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        Ok(UserPreference { id: eid, preference_key, preference_value, confidence, evidence: Some(evidence), updated_at: now })
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute("INSERT INTO user_preferences (id,preference_key,preference_value,confidence,evidence,updated_at) VALUES (?1,?2,?3,?4,?5,?6)", rusqlite::params![id,preference_key,preference_value,confidence,evidence,now]).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        Ok(UserPreference { id, preference_key, preference_value, confidence, evidence: Some(evidence), updated_at: now })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn valid_memory_types() { for mt in ALLOWED_MEMORY_TYPES { assert!(validate_memory_type(mt).is_ok()); } }
    #[test] fn invalid_memory_type() { assert!(validate_memory_type("bad").is_err()); assert!(validate_memory_type("").is_err()); }
    #[test] fn valid_layers() { for l in &["L1","L2","L3","L4","l1"] { assert!(validate_layer(l).is_ok()); } }
    #[test] fn invalid_layers() { assert!(validate_layer("L5").is_err()); assert!(validate_layer("").is_err()); }
    #[test] fn valid_confidence() { assert!(validate_confidence(0.0).is_ok()); assert!(validate_confidence(1.0).is_ok()); }
    #[test] fn invalid_confidence() { assert!(validate_confidence(1.1).is_err()); assert!(validate_confidence(-0.1).is_err()); }
    #[test] fn valid_version() { assert!(validate_version(1).is_ok()); assert!(validate_version(100).is_ok()); }
    #[test] fn invalid_version() { assert!(validate_version(0).is_err()); }

    #[test]
    fn layer_scope_combos() {
        assert!(validate_layer_scope(&MemoryLayer::L1, &MemoryScope::Session).is_ok());
        assert!(validate_layer_scope(&MemoryLayer::L2, &MemoryScope::Project).is_ok());
        assert!(validate_layer_scope(&MemoryLayer::L1, &MemoryScope::Project).is_err());
        assert!(validate_layer_scope(&MemoryLayer::L2, &MemoryScope::Session).is_err());
    }
}
