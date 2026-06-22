use crate::models::{ExportRecord, MemoryLayer, MemoryScope, ProjectMemory};
use crate::AppState;
use tauri::State;

fn get_exports_dir() -> Result<std::path::PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not determine data directory".to_string())?
        .join("game-agent-studio")
        .join("exports");
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(data_dir)
}

fn sanitize_project_name(name: &str) -> String {
    const WINDOWS_RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    let mut sanitized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == ' ' {
            sanitized.push(ch);
        } else if ch as u32 >= 0x80 {
            sanitized.push(ch);
        }
    }
    sanitized = sanitized.trim().to_string();
    sanitized = sanitized.replace(' ', "-");

    if sanitized.is_empty() {
        sanitized = "untitled".to_string();
    }

    let upper = sanitized.to_uppercase();
    let stem = sanitized
        .split('.')
        .next()
        .unwrap_or(&sanitized)
        .to_uppercase();
    if WINDOWS_RESERVED.contains(&upper.as_str())
        || WINDOWS_RESERVED.contains(&stem.as_str())
    {
        sanitized = format!("_{}", sanitized);
    }

    sanitized
}

fn verify_path_within_exports(file_path: &std::path::Path) -> Result<(), String> {
    let canonical_file = file_path
        .canonicalize()
        .map_err(|e| crate::models::sanitize_error(format!("Path error: {}", e)))?;
    let exports_dir = get_exports_dir()?;
    let canonical_exports = exports_dir
        .canonicalize()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    if !canonical_file.starts_with(&canonical_exports) {
        return Err("Export path is outside the allowed exports directory".to_string());
    }
    Ok(())
}

fn is_sensitive_memory_type(memory_type: &str) -> bool {
    memory_type == "qa_review" || memory_type == "system_internal"
}

#[tauri::command]
pub fn export_markdown(
    state: State<AppState>,
    project_id: String,
) -> Result<ExportRecord, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let mut stmt = db
        .prepare(
            "SELECT id, project_id, memory_type, key, value, layer, scope, source, confidence, version, provenance, created_at, updated_at FROM project_memory WHERE project_id = ?1 ORDER BY memory_type, key ASC",
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let all_entries: Vec<ProjectMemory> = stmt
        .query_map(rusqlite::params![project_id], |row| {
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
                layer: match layer_str.to_lowercase().as_str() {
                    "l4" => MemoryLayer::L4,
                    "l3" => MemoryLayer::L3,
                    "l2" => MemoryLayer::L2,
                    _ => MemoryLayer::L1,
                },
                scope: match scope_str.to_lowercase().as_str() {
                    "session" => MemoryScope::Session,
                    "global" => MemoryScope::Global,
                    _ => MemoryScope::Project,
                },
                source: row.get(7).ok(),
                confidence: confidence_val,
                version: version_val,
                provenance: row.get(10).ok(),
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .collect::<Result<Vec<ProjectMemory>, _>>()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let entries: Vec<&ProjectMemory> = all_entries
        .iter()
        .filter(|e| !is_sensitive_memory_type(&e.memory_type))
        .collect();

    let project_name: String = db
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Untitled Project".to_string());

    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", project_name));
    md.push_str(&format!(
        "*Exported on {}*\n\n",
        chrono::Utc::now().to_rfc3339()
    ));

    let mut current_type = String::new();
    for entry in &entries {
        if entry.memory_type != current_type {
            current_type = entry.memory_type.clone();
            md.push_str(&format!(
                "## {}\n\n",
                current_type.replace('_', " ").to_uppercase()
            ));
        }
        md.push_str(&format!("### {}\n\n", entry.key));
        md.push_str(&format!("{}\n\n", entry.value));
    }

    if entries.is_empty() {
        md.push_str("*No game design content recorded yet.*\n");
    }

    let exports_dir = get_exports_dir()?;
    let safe_name = sanitize_project_name(&project_name);
    let filename = format!(
        "{}-{}.md",
        safe_name,
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let file_path = exports_dir.join(&filename);
    verify_path_within_exports(&file_path)?;

    std::fs::write(&file_path, md)
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let file_path_str = file_path.to_string_lossy().to_string();

    db.execute(
        "INSERT INTO exports (id, project_id, export_type, file_path, created_at) VALUES (?1, ?2, 'markdown', ?3, ?4)",
        rusqlite::params![id, project_id, file_path_str, now],
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(ExportRecord {
        id,
        project_id,
        export_type: "markdown".to_string(),
        file_path: file_path_str,
        created_at: now,
    })
}

#[tauri::command]
pub fn export_json(
    state: State<AppState>,
    project_id: String,
) -> Result<ExportRecord, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let project_name: String = db
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Untitled Project".to_string());

    let mut memory_stmt = db
        .prepare(
            "SELECT id, project_id, memory_type, key, value, layer, scope, source, confidence, version, provenance, created_at, updated_at FROM project_memory WHERE project_id = ?1 ORDER BY memory_type, key ASC",
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let all_memories: Vec<ProjectMemory> = memory_stmt
        .query_map(rusqlite::params![project_id], |row| {
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
                layer: match layer_str.to_lowercase().as_str() {
                    "l4" => MemoryLayer::L4,
                    "l3" => MemoryLayer::L3,
                    "l2" => MemoryLayer::L2,
                    _ => MemoryLayer::L1,
                },
                scope: match scope_str.to_lowercase().as_str() {
                    "session" => MemoryScope::Session,
                    "global" => MemoryScope::Global,
                    _ => MemoryScope::Project,
                },
                source: row.get(7).ok(),
                confidence: confidence_val,
                version: version_val,
                provenance: row.get(10).ok(),
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .collect::<Result<Vec<ProjectMemory>, _>>()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let memories: Vec<&ProjectMemory> = all_memories
        .iter()
        .filter(|m| !is_sensitive_memory_type(&m.memory_type))
        .collect();

    let export_json = serde_json::json!({
        "project_name": project_name,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "memories": memories.iter().map(|m| {
            serde_json::json!({
                "memory_type": m.memory_type,
                "key": m.key,
                "value": m.value,
                "layer": m.layer,
                "scope": m.scope,
                "confidence": m.confidence,
                "version": m.version,
                "updated_at": m.updated_at,
            })
        }).collect::<Vec<_>>(),
    });

    let exports_dir = get_exports_dir()?;
    let safe_name = sanitize_project_name(&project_name);
    let filename = format!(
        "{}-{}.json",
        safe_name,
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let file_path = exports_dir.join(&filename);
    verify_path_within_exports(&file_path)?;

    std::fs::write(
        &file_path,
        serde_json::to_string_pretty(&export_json)
            .map_err(|e| crate::models::sanitize_error(e.to_string()))?,
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let file_path_str = file_path.to_string_lossy().to_string();

    db.execute(
        "INSERT INTO exports (id, project_id, export_type, file_path, created_at) VALUES (?1, ?2, 'json', ?3, ?4)",
        rusqlite::params![id, project_id, file_path_str, now],
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(ExportRecord {
        id,
        project_id,
        export_type: "json".to_string(),
        file_path: file_path_str,
        created_at: now,
    })
}

#[tauri::command]
pub fn get_exports(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<ExportRecord>, String> {
    let db = state
        .db
        .lock()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db
        .prepare(
            "SELECT id, project_id, export_type, file_path, created_at FROM exports WHERE project_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let records = stmt
        .query_map(rusqlite::params![project_id], |row| {
            Ok(ExportRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                export_type: row.get(2)?,
                file_path: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .collect::<Result<Vec<ExportRecord>, _>>()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(records)
}
