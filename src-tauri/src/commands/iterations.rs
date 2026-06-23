use crate::commands::events;
use crate::models::{ImprovementProposal, ProposalStatus, ProposalType};
use crate::AppState;
use tauri::State;

fn parse_proposal_type(s: &str) -> ProposalType {
    match s {
        "workflow_improvement" => ProposalType::WorkflowImprovement,
        "prompt_improvement" => ProposalType::PromptImprovement,
        "code_improvement" => ProposalType::CodeImprovement,
        "export_template_fix" => ProposalType::ExportTemplateFix,
        "safety_enhancement" => ProposalType::SafetyEnhancement,
        "ui_ux_improvement" => ProposalType::UiUxImprovement,
        "data_model_refinement" => ProposalType::DataModelRefinement,
        _ => ProposalType::Other(s.to_string()),
    }
}

fn proposal_type_to_str(t: &ProposalType) -> String {
    match t {
        ProposalType::WorkflowImprovement => "workflow_improvement".into(),
        ProposalType::PromptImprovement => "prompt_improvement".into(),
        ProposalType::CodeImprovement => "code_improvement".into(),
        ProposalType::ExportTemplateFix => "export_template_fix".into(),
        ProposalType::SafetyEnhancement => "safety_enhancement".into(),
        ProposalType::UiUxImprovement => "ui_ux_improvement".into(),
        ProposalType::DataModelRefinement => "data_model_refinement".into(),
        ProposalType::Other(s) => s.clone(),
    }
}

fn parse_proposal_status(s: &str) -> Result<ProposalStatus, String> {
    match s {
        "draft" => Ok(ProposalStatus::Draft),
        "proposed" => Ok(ProposalStatus::Proposed),
        "accepted" => Ok(ProposalStatus::Accepted),
        "rejected" => Ok(ProposalStatus::Rejected),
        "implemented" => Ok(ProposalStatus::Implemented),
        "superseded" => Ok(ProposalStatus::Superseded),
        _ => Err(crate::models::sanitize_error(format!("Invalid proposal status: {}", s))),
    }
}

fn proposal_status_to_str(s: &ProposalStatus) -> &str {
    match s {
        ProposalStatus::Draft => "draft", ProposalStatus::Proposed => "proposed",
        ProposalStatus::Accepted => "accepted", ProposalStatus::Rejected => "rejected",
        ProposalStatus::Implemented => "implemented", ProposalStatus::Superseded => "superseded",
    }
}

fn is_valid_status_transition(from: &ProposalStatus, to: &ProposalStatus) -> bool {
    matches!((from, to),
        (ProposalStatus::Draft, ProposalStatus::Proposed)
        | (ProposalStatus::Proposed, ProposalStatus::Accepted)
        | (ProposalStatus::Proposed, ProposalStatus::Rejected)
        | (ProposalStatus::Accepted, ProposalStatus::Implemented)
        | (ProposalStatus::Accepted, ProposalStatus::Superseded)
        | (ProposalStatus::Rejected, ProposalStatus::Superseded))
}

fn validate_risk_level(risk_level: &str) -> Result<String, String> {
    match risk_level {
        "low" | "medium" | "high" | "critical" => Ok(risk_level.to_string()),
        _ => Err(crate::models::sanitize_error(format!("Invalid risk_level: {}", risk_level))),
    }
}

fn validate_not_empty(field_name: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { Err(format!("{} must not be empty", field_name)) }
    else { Ok(trimmed.to_string()) }
}

fn requires_approval(pt: &ProposalType) -> bool {
    matches!(pt, ProposalType::CodeImprovement | ProposalType::PromptImprovement
        | ProposalType::SafetyEnhancement | ProposalType::ExportTemplateFix
        | ProposalType::DataModelRefinement)
}

fn log_proposal_event(
    db: &rusqlite::Connection, event_type: &str, proposal_id: &str,
    proposal_type: &str, old_status: &str, new_status: &str,
    risk_level: &str, target_area: &str, requires_human_approval: bool, actor: &str,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let cid = uuid::Uuid::new_v4().to_string();
    let payload = crate::models::sanitize_error(serde_json::json!({
        "proposal_id": proposal_id, "proposal_type": proposal_type,
        "old_status": old_status, "new_status": new_status,
        "risk_level": risk_level, "target_area": target_area,
        "requires_human_approval": requires_human_approval,
    }).to_string());
    db.execute(
        "INSERT INTO events (id, project_id, run_id, actor, event_type, event_data, severity, correlation_id, redaction_level, created_at) VALUES (?1,NULL,NULL,?2,?3,?4,'info',?5,NULL,?6)",
        rusqlite::params![id, actor, event_type, payload, cid, now],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    Ok(())
}

fn read_proposal(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImprovementProposal> {
    let pt: String = row.get(1)?; let st: String = row.get(5)?; let rha: i32 = row.get(6)?;
    let status = parse_proposal_status(&st)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))))?;
    Ok(ImprovementProposal { id: row.get(0)?, proposal_type: parse_proposal_type(&pt), summary: row.get(2)?, evidence: row.get(3).ok(), risk_level: row.get(4).ok(), status, requires_human_approval: rha != 0, target_area: row.get(7).ok(), proposed_change: row.get(8).ok(), created_at: row.get(9)?, reviewed_at: row.get(10).ok() })
}

// ════════════════════════════════════════════════════════════
// Core logic — shared by Tauri commands and tests
// ════════════════════════════════════════════════════════════

fn create_proposal_impl(
    db: &mut rusqlite::Connection, proposal_type: &str,
    summary: &str, evidence: &str, risk: &str,
    target_area: &str, proposed_change: &str,
) -> Result<ImprovementProposal, String> {
    let summary = validate_not_empty("summary", summary)?;
    let evidence = validate_not_empty("evidence", evidence)?;
    let risk = validate_risk_level(risk)?;
    let target_area = validate_not_empty("target_area", target_area)?;
    let proposed_change = validate_not_empty("proposed_change", proposed_change)?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let pt = parse_proposal_type(proposal_type);
    let rha = requires_approval(&pt);

    let tx = db.transaction().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    tx.execute(
        "INSERT INTO improvement_proposals (id, proposal_type, summary, evidence, risk_level, status, requires_human_approval, target_area, proposed_change, created_at) VALUES (?1,?2,?3,?4,?5,'proposed',?6,?7,?8,?9)",
        rusqlite::params![id, proposal_type_to_str(&pt), summary, evidence, risk, rha as i32, target_area, proposed_change, now],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    log_proposal_event(&tx, events::EVENT_PROPOSAL_CREATED, &id,
        &proposal_type_to_str(&pt), "", "proposed", &risk, &target_area, rha, "system")?;

    tx.commit().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(ImprovementProposal {
        id, proposal_type: pt, summary, evidence: Some(evidence), risk_level: Some(risk),
        status: ProposalStatus::Proposed, requires_human_approval: rha,
        target_area: Some(target_area), proposed_change: Some(proposed_change),
        created_at: now, reviewed_at: None,
    })
}

fn review_proposal_impl(
    db: &mut rusqlite::Connection, proposal_id: &str, new_status: &str,
) -> Result<ImprovementProposal, String> {
    let new_ps = parse_proposal_status(new_status)?;
    let now = chrono::Utc::now().to_rfc3339();

    // Read current under transaction to ensure no race
    let current: ImprovementProposal = db.query_row(
        "SELECT id, proposal_type, summary, evidence, risk_level, status, requires_human_approval, target_area, proposed_change, created_at, reviewed_at FROM improvement_proposals WHERE id=?1",
        rusqlite::params![proposal_id], |r| read_proposal(r),
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    if !is_valid_status_transition(&current.status, &new_ps) {
        return Err(crate::models::sanitize_error(format!(
            "Invalid status transition: {:?} -> {:?}", current.status, new_ps)));
    }

    let tx = db.transaction().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let rows = tx.execute(
        "UPDATE improvement_proposals SET status=?1, reviewed_at=?2 WHERE id=?3",
        rusqlite::params![proposal_status_to_str(&new_ps), now, proposal_id],
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    if rows == 0 { return Err("Proposal not found".to_string()); }

    log_proposal_event(&tx, events::EVENT_PROPOSAL_REVIEWED, proposal_id,
        &proposal_type_to_str(&current.proposal_type),
        &proposal_status_to_str(&current.status),
        &proposal_status_to_str(&new_ps),
        current.risk_level.as_deref().unwrap_or("unknown"),
        current.target_area.as_deref().unwrap_or(""),
        current.requires_human_approval, "user")?;

    tx.commit().map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    // Read back after commit
    db.query_row(
        "SELECT id, proposal_type, summary, evidence, risk_level, status, requires_human_approval, target_area, proposed_change, created_at, reviewed_at FROM improvement_proposals WHERE id=?1",
        rusqlite::params![proposal_id], |r| read_proposal(r),
    ).map_err(|e| crate::models::sanitize_error(e.to_string()))
}

// ════════════════════════════════════════════════════════════
// Tauri Commands — thin wrappers
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn create_improvement_proposal(
    state: State<AppState>, proposal_type: String, summary: String,
    evidence: Option<String>, risk_level: Option<String>,
    target_area: Option<String>, proposed_change: Option<String>,
) -> Result<ImprovementProposal, String> {
    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    create_proposal_impl(&mut *db, &proposal_type,
        &summary, evidence.as_deref().unwrap_or(""),
        risk_level.as_deref().unwrap_or(""),
        target_area.as_deref().unwrap_or(""),
        proposed_change.as_deref().unwrap_or(""))
}

#[tauri::command]
pub fn list_improvement_proposals(
    state: State<AppState>, status: Option<String>,
) -> Result<Vec<ImprovementProposal>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let sql = match status {
        Some(_) => format!("SELECT id, proposal_type, summary, evidence, risk_level, status, requires_human_approval, target_area, proposed_change, created_at, reviewed_at FROM improvement_proposals WHERE status=?1 ORDER BY created_at DESC"),
        None => "SELECT id, proposal_type, summary, evidence, risk_level, status, requires_human_approval, target_area, proposed_change, created_at, reviewed_at FROM improvement_proposals ORDER BY created_at DESC".into(),
    };
    let mut stmt = db.prepare(&sql).map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let rows = match status {
        Some(s) => { let r = stmt.query_map(rusqlite::params![s], |r| read_proposal(r)).map_err(|e| crate::models::sanitize_error(e.to_string()))?; r.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))? }
        None => { let r = stmt.query_map([], |r| read_proposal(r)).map_err(|e| crate::models::sanitize_error(e.to_string()))?; r.collect::<Result<Vec<_>,_>>().map_err(|e| crate::models::sanitize_error(e.to_string()))? }
    };
    Ok(rows)
}

#[tauri::command]
pub fn review_improvement_proposal(
    state: State<AppState>, proposal_id: String, new_status: String,
) -> Result<ImprovementProposal, String> {
    let mut db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    review_proposal_impl(&mut db, &proposal_id, &new_status)
}

// ════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn valid_status_transitions() {
        assert!(is_valid_status_transition(&ProposalStatus::Draft, &ProposalStatus::Proposed));
        assert!(is_valid_status_transition(&ProposalStatus::Proposed, &ProposalStatus::Accepted));
        assert!(is_valid_status_transition(&ProposalStatus::Proposed, &ProposalStatus::Rejected));
        assert!(is_valid_status_transition(&ProposalStatus::Accepted, &ProposalStatus::Implemented));
        assert!(!is_valid_status_transition(&ProposalStatus::Draft, &ProposalStatus::Accepted));
        assert!(!is_valid_status_transition(&ProposalStatus::Proposed, &ProposalStatus::Draft));
    }

    #[test] fn risk_level_validation() {
        assert!(validate_risk_level("low").is_ok()); assert!(validate_risk_level("medium").is_ok());
        assert!(validate_risk_level("high").is_ok()); assert!(validate_risk_level("critical").is_ok());
        assert!(validate_risk_level("invalid").is_err()); assert!(validate_risk_level("").is_err());
    }

    #[test] fn status_parsing() {
        assert!(parse_proposal_status("proposed").is_ok());
        assert!(parse_proposal_status("accepted").is_ok());
        assert!(parse_proposal_status("bogus").is_err());
    }

    #[test] fn approval_required_for_dangerous_types() {
        assert!(requires_approval(&ProposalType::CodeImprovement));
        assert!(requires_approval(&ProposalType::PromptImprovement));
        assert!(requires_approval(&ProposalType::SafetyEnhancement));
        assert!(requires_approval(&ProposalType::ExportTemplateFix));
        assert!(requires_approval(&ProposalType::DataModelRefinement));
        assert!(!requires_approval(&ProposalType::WorkflowImprovement));
        assert!(!requires_approval(&ProposalType::UiUxImprovement));
    }

    // ── command-level tests with in-memory DB ──

    fn setup_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrations::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn create_proposal_missing_summary() {
        let mut db = setup_db();
        let r = create_proposal_impl(&mut db, "prompt_improvement", "  ", "ev", "high", "ta", "pc");
        assert!(r.is_err());
    }

    #[test]
    fn create_proposal_missing_evidence() {
        let mut db = setup_db();
        let r = create_proposal_impl(&mut db, "prompt_improvement", "summary", "  ", "high", "ta", "pc");
        assert!(r.is_err());
    }

    #[test]
    fn create_proposal_missing_risk_level() {
        let mut db = setup_db();
        let r = create_proposal_impl(&mut db, "prompt_improvement", "summary", "ev", "", "ta", "pc");
        assert!(r.is_err());
    }

    #[test]
    fn create_proposal_missing_target_area() {
        let mut db = setup_db();
        let r = create_proposal_impl(&mut db, "prompt_improvement", "summary", "ev", "high", "  ", "pc");
        assert!(r.is_err());
    }

    #[test]
    fn create_proposal_missing_proposed_change() {
        let mut db = setup_db();
        let r = create_proposal_impl(&mut db, "prompt_improvement", "summary", "ev", "high", "ta", "  ");
        assert!(r.is_err());
    }

    #[test]
    fn create_proposal_writes_to_db() {
        let mut db = setup_db();
        let p = create_proposal_impl(&mut db, "prompt_improvement", "test summary", "evidence", "high", "target", "change").unwrap();
        assert_eq!(p.summary, "test summary");
        assert!(p.requires_human_approval);
        let count: i32 = db.query_row("SELECT COUNT(*) FROM improvement_proposals", [], |r| r.get(0)).unwrap();
        assert!(count >= 1);
        let evt: i32 = db.query_row("SELECT COUNT(*) FROM events WHERE event_type='proposal_created'", [], |r| r.get(0)).unwrap();
        assert!(evt >= 1);
    }

    #[test]
    fn review_proposal_invalid_transition_fails() {
        let mut db = setup_db();
        let p = create_proposal_impl(&mut db, "ui_ux_improvement", "summary", "ev", "low", "ta", "pc").unwrap();
        let r = review_proposal_impl(&mut db, &p.id, "implemented");
        assert!(r.is_err());
    }

    #[test]
    fn review_proposal_updates_status_and_writes_event() {
        let mut db = setup_db();
        let p = create_proposal_impl(&mut db, "workflow_improvement", "summary", "ev", "medium", "ta", "pc").unwrap();
        let reviewed = review_proposal_impl(&mut db, &p.id, "accepted").unwrap();
        assert_eq!(reviewed.status, ProposalStatus::Accepted);
        assert!(reviewed.reviewed_at.is_some());
        let evt: i32 = db.query_row("SELECT COUNT(*) FROM events WHERE event_type='proposal_reviewed'", [], |r| r.get(0)).unwrap();
        assert!(evt >= 1);
    }
}
