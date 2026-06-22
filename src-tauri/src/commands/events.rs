use crate::models::{Event, EventSeverity};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn log_event(
    state: State<AppState>,
    project_id: Option<String>,
    event_type: String,
    event_data: String,
) -> Result<Event, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(
        "INSERT INTO events (id, project_id, event_type, event_data, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, project_id, event_type, event_data, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Event {
        id,
        project_id,
        run_id: None,
        actor: None,
        event_type,
        event_data,
        severity: EventSeverity::Info,
        correlation_id: None,
        redaction_level: None,
        created_at: now,
    })
}

#[tauri::command]
pub fn get_events(
    state: State<AppState>,
    project_id: Option<String>,
    limit: u32,
) -> Result<Vec<Event>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let events = match project_id {
        Some(pid) => {
            let mut stmt = db
                .prepare("SELECT id, project_id, event_type, event_data, created_at FROM events WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map(rusqlite::params![pid, limit], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    run_id: None,
                    actor: None,
                    event_type: row.get(2)?,
                    event_data: row.get(3)?,
                    severity: EventSeverity::Info,
                    correlation_id: None,
                    redaction_level: None,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<Event>, _>>()
                .map_err(|e| e.to_string())?
        }
        None => {
            let mut stmt = db
                .prepare("SELECT id, project_id, event_type, event_data, created_at FROM events ORDER BY created_at DESC LIMIT ?1")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map(rusqlite::params![limit], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    run_id: None,
                    actor: None,
                    event_type: row.get(2)?,
                    event_data: row.get(3)?,
                    severity: EventSeverity::Info,
                    correlation_id: None,
                    redaction_level: None,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<Event>, _>>()
                .map_err(|e| e.to_string())?
        }
    };

    Ok(events)
}
