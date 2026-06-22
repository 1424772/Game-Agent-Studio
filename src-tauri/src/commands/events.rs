use crate::models::{Event, EventSeverity};
use crate::AppState;
use tauri::State;

pub const EVENT_WORKFLOW_START: &str = "workflow_start";
pub const EVENT_WORKFLOW_COMPLETE: &str = "workflow_complete";
pub const EVENT_WORKFLOW_FAILED: &str = "workflow_failed";
pub const EVENT_STEP_START: &str = "step_start";
pub const EVENT_STEP_COMPLETE: &str = "step_complete";
pub const EVENT_STEP_FAILED: &str = "step_failed";
pub const EVENT_MEMORY_SAVED: &str = "memory_saved";
pub const EVENT_MESSAGE_ACCEPTED: &str = "output_accepted";
pub const EVENT_MESSAGE_REJECTED: &str = "output_rejected";
pub const EVENT_MESSAGE_EDITED: &str = "output_edited";
pub const EVENT_EXPORT_CREATED: &str = "export_created";
pub const EVENT_PROJECT_CREATED: &str = "project_created";
pub const EVENT_MODEL_CONFIG_SAVED: &str = "model_config_saved";
pub const EVENT_WORKFLOW_RUN_COMPLETE: &str = "agent_workflow_completed";
pub const EVENT_WORKFLOW_RUN_FAILED: &str = "agent_workflow_failed";
pub const EVENT_PROPOSAL_CREATED: &str = "proposal_created";
pub const EVENT_PROPOSAL_REVIEWED: &str = "proposal_reviewed";

fn parse_severity(s: &Option<String>) -> EventSeverity {
    match s.as_ref().map(|v| v.to_lowercase()) {
        Some(ref v) if v.as_str() == "debug" => EventSeverity::Debug,
        Some(ref v) if v.as_str() == "info" => EventSeverity::Info,
        Some(ref v) if v.as_str() == "warning" => EventSeverity::Warning,
        Some(ref v) if v.as_str() == "error" => EventSeverity::Error,
        Some(ref v) if v.as_str() == "critical" => EventSeverity::Critical,
        _ => EventSeverity::Info,
    }
}

fn severity_to_str(s: &EventSeverity) -> &str {
    match s { EventSeverity::Debug=>"debug",EventSeverity::Info=>"info",EventSeverity::Warning=>"warning",EventSeverity::Error=>"error",EventSeverity::Critical=>"critical" }
}

fn read_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    let sv: String = row.get(6)?;
    Ok(Event { id: row.get(0)?, project_id: row.get(1)?, run_id: row.get(2)?, actor: row.get(3)?, event_type: row.get(4)?, event_data: row.get(5)?, severity: parse_severity(&Some(sv)), correlation_id: row.get(7)?, redaction_level: row.get(8)?, created_at: row.get(9)? })
}

#[tauri::command]
pub fn log_event(
    state: State<AppState>, project_id: Option<String>, run_id: Option<String>,
    actor: Option<String>, event_type: String, event_data: String,
    severity: Option<String>, correlation_id: Option<String>, redaction_level: Option<String>,
) -> Result<Event, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let parsed_severity = parse_severity(&severity);
    let sanitized_data = crate::models::sanitize_error(event_data);
    db.execute(
        "INSERT INTO events (id,project_id,run_id,actor,event_type,event_data,severity,correlation_id,redaction_level,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        rusqlite::params![id, project_id, run_id, actor, event_type, sanitized_data, severity_to_str(&parsed_severity), correlation_id, redaction_level, now],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(Event { id, project_id, run_id, actor, event_type, event_data: sanitized_data, severity: parsed_severity, correlation_id, redaction_level, created_at: now })
}

#[tauri::command]
pub fn get_events(
    state: State<AppState>,
    project_id: Option<String>,
    run_id: Option<String>,
    correlation_id: Option<String>,
    event_type: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<Event>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut sql = String::from("SELECT id,project_id,run_id,actor,event_type,event_data,severity,correlation_id,redaction_level,created_at FROM events WHERE 1=1");
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref pid) = project_id { sql.push_str(" AND project_id=?"); params.push(Box::new(pid.clone())); }
    if let Some(ref rid) = run_id { sql.push_str(" AND run_id=?"); params.push(Box::new(rid.clone())); }
    if let Some(ref cid) = correlation_id { sql.push_str(" AND correlation_id=?"); params.push(Box::new(cid.clone())); }
    if let Some(ref et) = event_type { sql.push_str(" AND event_type=?"); params.push(Box::new(et.clone())); }

    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    let lim = limit.unwrap_or(100);
    params.push(Box::new(lim as i64));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.prepare(&sql).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let rows = stmt.query_map(param_refs.as_slice(), |r| read_event_row(r)).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    rows.collect::<Result<Vec<Event>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))
}
