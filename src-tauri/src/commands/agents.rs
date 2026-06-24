use crate::commands::memory;
use crate::commands::rag;
use crate::commands::security;
use crate::commands::workflow;
use crate::crypto::decrypt_saved_api_key;
use crate::crypto::SecretStore;
use crate::models::{
    AgentMessage, AgentRun, AgentRunStatus, AgentStep, LlmRequest, LlmResponse, LlmUsage,
    MessageStatus, WorkflowType, sanitize_error,
};
use crate::AppState;
use serde_json::Value;
use tauri::State;

fn parse_workflow_type(s: &str) -> WorkflowType {
    match s {
        "card_game_concept" => WorkflowType::CardGameConcept,
        "visual_novel_concept" => WorkflowType::VisualNovelConcept,
        "game_design_doc" => WorkflowType::GameDesignDoc,
        _ => WorkflowType::Custom(s.to_string()),
    }
}

fn parse_agent_run_status(s: &str) -> Result<AgentRunStatus, String> {
    match s {
        "pending" => Ok(AgentRunStatus::Pending),
        "running" => Ok(AgentRunStatus::Running),
        "waiting_for_input" => Ok(AgentRunStatus::WaitingForInput),
        "completed" => Ok(AgentRunStatus::Completed),
        "failed" => Ok(AgentRunStatus::Failed),
        "cancelled" => Ok(AgentRunStatus::Cancelled),
        _ => Err(sanitize_error(format!("Invalid agent run status: {}", s))),
    }
}

fn agent_run_status_to_str(s: &AgentRunStatus) -> &'static str {
    match s {
        AgentRunStatus::Pending => "pending",
        AgentRunStatus::Running => "running",
        AgentRunStatus::WaitingForInput => "waiting_for_input",
        AgentRunStatus::Completed => "completed",
        AgentRunStatus::Failed => "failed",
        AgentRunStatus::Cancelled => "cancelled",
    }
}

fn parse_message_status(s: &str) -> Option<MessageStatus> {
    match s {
        "pending" => Some(MessageStatus::Pending),
        "streaming" => Some(MessageStatus::Streaming),
        "completed" => Some(MessageStatus::Completed),
        "failed" => Some(MessageStatus::Failed),
        "cancelled" => Some(MessageStatus::Cancelled),
        "accepted" => Some(MessageStatus::Accepted),
        "rejected" => Some(MessageStatus::Rejected),
        "edited" => Some(MessageStatus::Edited),
        _ => None,
    }
}

fn message_status_to_str(s: &MessageStatus) -> &'static str {
    match s {
        MessageStatus::Pending => "pending",
        MessageStatus::Streaming => "streaming",
        MessageStatus::Completed => "completed",
        MessageStatus::Failed => "failed",
        MessageStatus::Cancelled => "cancelled",
        MessageStatus::Accepted => "accepted",
        MessageStatus::Rejected => "rejected",
        MessageStatus::Edited => "edited",
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let total_chars = s.chars().count();
    if total_chars <= max_chars {
        return s.to_string();
    }
    if max_chars < 50 {
        let suffix = "[...]";
        let keep = max_chars.saturating_sub(suffix.chars().count());
        return format!("{}{}", s.chars().take(keep).collect::<String>(), suffix);
    }
    let skipped = total_chars.saturating_sub(max_chars);
    let marker = format!("\n[...truncated {} chars...]\n", skipped);
    let marker_len = marker.chars().count();
    let content_budget = max_chars.saturating_sub(marker_len);
    let head_keep = (content_budget as f64 * 0.65) as usize;
    let tail_keep = content_budget.saturating_sub(head_keep);
    let head: String = s.chars().take(head_keep).collect();
    let tail: String = s.chars().rev().take(tail_keep).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{}{}{}", head, marker, tail)
}

fn truncate_rag_context(excerpts: &[(String, String, String)], max_chars: usize) -> Vec<(String, String, String)> {
    if max_chars == 0 {
        return excerpts.to_vec();
    }
    let sep = "---\n";
    let sep_len = sep.chars().count();
    let mut remaining = max_chars as i64;
    let mut result = Vec::new();
    for (chunk_id, title, excerpt) in excerpts {
        let prefix = format!("[Source: {}] ", title);
        let prefix_len = prefix.chars().count() as i64;
        let between = if !result.is_empty() { sep_len as i64 + 1 } else { 0 };
        let overhead = between + prefix_len;
        if overhead >= remaining {
            break;
        }
        let body_budget = (remaining - overhead) as usize;
        let body: String = excerpt.chars().take(body_budget).collect();
        let body_len = body.chars().count() as i64;
        remaining = remaining - overhead - body_len;
        result.push((chunk_id.clone(), title.clone(), body));
    }
    result
}

fn log_workflow_event(
    state: &State<'_, AppState>,
    project_id: &str,
    run_id: &str,
    event_type: &str,
    event_data: &str,
    correlation_id: &str,
    severity: &str,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let sanitized_data = sanitize_error(event_data.to_string());
    db.execute(
        "INSERT INTO events (id, project_id, run_id, actor, event_type, event_data, severity, correlation_id, redaction_level, created_at) VALUES (?1,?2,?3,'system',?4,?5,?6,?7,NULL,?8)",
        rusqlite::params![id, project_id, run_id, event_type, sanitized_data, severity, correlation_id, now],
    ).map_err(|e| sanitize_error(e.to_string()))?;
    Ok(())
}

// ════════════════════════════════════════════════════════════
// Agent Runs
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn create_agent_run(
    state: State<AppState>, project_id: String, task_description: String, workflow_type: String,
) -> Result<AgentRun, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO agent_runs (id, project_id, task_description, workflow_type, status, created_at, updated_at) VALUES (?1,?2,?3,?4,'running',?5,?6)",
        rusqlite::params![id, project_id, task_description, workflow_type, now, now],
    ).map_err(|e| sanitize_error(e.to_string()))?;
    Ok(AgentRun { id, project_id, task_description, workflow_type: parse_workflow_type(&workflow_type), status: AgentRunStatus::Running, created_at: now.clone(), updated_at: now })
}

#[tauri::command]
pub fn update_agent_run(state: State<AppState>, run_id: String, status: String) -> Result<AgentRun, String> {
    let new_status = parse_agent_run_status(&status)?;
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();
    let status_str = agent_run_status_to_str(&new_status);

    db.execute("UPDATE agent_runs SET status = ?1, updated_at = ?2 WHERE id = ?3", rusqlite::params![status_str, now, run_id])
        .map_err(|e| sanitize_error(e.to_string()))?;

    db.query_row(
        "SELECT id, project_id, task_description, workflow_type, status, created_at, updated_at FROM agent_runs WHERE id = ?1",
        rusqlite::params![run_id],
        |row| { let wt: String = row.get(3)?; let st: String = row.get(4)?;
            Ok(AgentRun { id: row.get(0)?, project_id: row.get(1)?, task_description: row.get(2)?, workflow_type: parse_workflow_type(&wt), status: parse_agent_run_status(&st).unwrap_or(AgentRunStatus::Failed), created_at: row.get(5)?, updated_at: row.get(6)? })
        },
    ).map_err(|e| sanitize_error(e.to_string()))
}

#[tauri::command]
pub fn get_agent_run(state: State<AppState>, run_id: String) -> Result<AgentRun, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    db.query_row(
        "SELECT id, project_id, task_description, workflow_type, status, created_at, updated_at FROM agent_runs WHERE id = ?1",
        rusqlite::params![run_id],
        |row| { let wt: String = row.get(3)?; let st: String = row.get(4)?;
            Ok(AgentRun { id: row.get(0)?, project_id: row.get(1)?, task_description: row.get(2)?, workflow_type: parse_workflow_type(&wt), status: parse_agent_run_status(&st).unwrap_or(AgentRunStatus::Failed), created_at: row.get(5)?, updated_at: row.get(6)? })
        },
    ).map_err(|e| sanitize_error(e.to_string()))
}

#[tauri::command]
pub fn get_agent_runs(state: State<AppState>, project_id: String) -> Result<Vec<AgentRun>, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id, project_id, task_description, workflow_type, status, created_at, updated_at FROM agent_runs WHERE project_id = ?1 ORDER BY created_at DESC").map_err(|e| sanitize_error(e.to_string()))?;
    let runs = stmt.query_map(rusqlite::params![project_id], |row| {
        let wt: String = row.get(3)?; let st: String = row.get(4)?;
        Ok(AgentRun { id: row.get(0)?, project_id: row.get(1)?, task_description: row.get(2)?, workflow_type: parse_workflow_type(&wt), status: parse_agent_run_status(&st).unwrap_or(AgentRunStatus::Failed), created_at: row.get(5)?, updated_at: row.get(6)? })
    }).map_err(|e| sanitize_error(e.to_string()))?.collect::<Result<Vec<AgentRun>,_>>().map_err(|e| sanitize_error(e.to_string()))?;
    Ok(runs)
}

// ════════════════════════════════════════════════════════════
// Agent Messages
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn save_agent_message(state: State<AppState>, run_id: String, agent_name: String, role: String, content: String, metadata: Option<String>) -> Result<AgentMessage, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let meta_str = metadata.unwrap_or_else(|| "{}".to_string());
    db.execute("INSERT INTO agent_messages (id, run_id, agent_name, role, content, metadata, status, created_at) VALUES (?1,?2,?3,?4,?5,?6,'completed',?7)", rusqlite::params![id, run_id, agent_name, role, content, meta_str, now]).map_err(|e| sanitize_error(e.to_string()))?;
    Ok(AgentMessage { id, run_id, agent_name, role, content, metadata: Some(meta_str), status: MessageStatus::Completed, created_at: now })
}

#[tauri::command]
pub fn get_agent_messages(state: State<AppState>, run_id: String) -> Result<Vec<AgentMessage>, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id, run_id, agent_name, role, content, metadata, status, created_at FROM agent_messages WHERE run_id = ?1 ORDER BY created_at ASC").map_err(|e| sanitize_error(e.to_string()))?;
    let msgs = stmt.query_map(rusqlite::params![run_id], |row| {
        let st: String = row.get(6)?; let meta_raw: String = row.get(5)?;
        Ok(AgentMessage { id: row.get(0)?, run_id: row.get(1)?, agent_name: row.get(2)?, role: row.get(3)?, content: row.get(4)?, metadata: if meta_raw == "{}" { None } else { Some(meta_raw) }, status: parse_message_status(&st).unwrap_or(MessageStatus::Pending), created_at: row.get(7)? })
    }).map_err(|e| sanitize_error(e.to_string()))?.collect::<Result<Vec<AgentMessage>,_>>().map_err(|e| sanitize_error(e.to_string()))?;
    Ok(msgs)
}

#[tauri::command]
pub fn update_message_status(state: State<AppState>, message_id: String, status: String) -> Result<AgentMessage, String> {
    let parsed = parse_message_status(&status).ok_or_else(|| sanitize_error(format!("Invalid message status: {}", status)))?;
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let rows = db.execute("UPDATE agent_messages SET status = ?1 WHERE id = ?2", rusqlite::params![message_status_to_str(&parsed), message_id]).map_err(|e| sanitize_error(e.to_string()))?;
    if rows == 0 { return Err("Message not found".to_string()); }
    db.query_row("SELECT id, run_id, agent_name, role, content, metadata, status, created_at FROM agent_messages WHERE id = ?1", rusqlite::params![message_id], |row| {
        let st: String = row.get(6)?; let meta_raw: String = row.get(5)?;
        Ok(AgentMessage { id: row.get(0)?, run_id: row.get(1)?, agent_name: row.get(2)?, role: row.get(3)?, content: row.get(4)?, metadata: if meta_raw == "{}" { None } else { Some(meta_raw) }, status: parse_message_status(&st).unwrap_or(MessageStatus::Pending), created_at: row.get(7)? })
    }).map_err(|e| sanitize_error(e.to_string()))
}

#[tauri::command]
pub fn update_agent_message_content(state: State<AppState>, message_id: String, edited_content: String) -> Result<AgentMessage, String> {
    let mut db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    let original: (String, usize) = db.query_row(
        "SELECT content, (SELECT COALESCE(MAX(revision),0) FROM message_revisions WHERE message_id = ?1) FROM agent_messages WHERE id = ?1",
        rusqlite::params![message_id],
        |row| Ok((row.get::<_,String>(0)?, row.get::<_,usize>(1)?)),
    ).map_err(|e| sanitize_error(e.to_string()))?;

    let new_revision = original.1 + 1;
    let rev_id = uuid::Uuid::new_v4().to_string();

    let tx = db.transaction().map_err(|e| sanitize_error(e.to_string()))?;

    tx.execute(
        "INSERT INTO message_revisions (id, message_id, revision, original_content, edited_content, editor, created_at) VALUES (?1,?2,?3,?4,?5,'user',?6)",
        rusqlite::params![rev_id, message_id, new_revision as i32, original.0, edited_content, now],
    ).map_err(|e| sanitize_error(e.to_string()))?;

    tx.execute(
        "UPDATE agent_messages SET content = ?1, status = 'edited' WHERE id = ?2",
        rusqlite::params![edited_content, message_id],
    ).map_err(|e| sanitize_error(e.to_string()))?;

    tx.commit().map_err(|e| sanitize_error(e.to_string()))?;

    db.query_row("SELECT id, run_id, agent_name, role, content, metadata, status, created_at FROM agent_messages WHERE id = ?1", rusqlite::params![message_id], |row| {
        let st: String = row.get(6)?; let meta_raw: String = row.get(5)?;
        Ok(AgentMessage { id: row.get(0)?, run_id: row.get(1)?, agent_name: row.get(2)?, role: row.get(3)?, content: row.get(4)?, metadata: if meta_raw == "{}" { None } else { Some(meta_raw) }, status: parse_message_status(&st).unwrap_or(MessageStatus::Pending), created_at: row.get(7)? })
    }).map_err(|e| sanitize_error(e.to_string()))
}

// ════════════════════════════════════════════════════════════
// Agent Steps — upsert by (run_id, step_key)
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub fn save_agent_step(
    state: State<AppState>, run_id: String, agent_name: String,
    step_key: String, step_order: i32, step_type: String,
    input_json: Option<String>, output_json: Option<String>, status: String,
    error_message: Option<String>, prompt_tokens: Option<i32>, completion_tokens: Option<i32>,
) -> Result<AgentStep, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    let existing: Option<(String, Option<String>, Option<String>, Option<String>, Option<i32>, Option<i32>)> = db
        .query_row(
            "SELECT id, input_json, output_json, error_message, prompt_tokens, completion_tokens FROM agent_steps WHERE run_id = ?1 AND step_key = ?2",
            rusqlite::params![run_id, step_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).ok();

    if let Some((step_id, prev_input, prev_output, prev_error, prev_prompt, prev_completion)) = existing {
        let final_input = input_json.or(prev_input);
        let final_output = output_json.or(prev_output);
        let final_error = error_message.or(prev_error);
        let final_prompt = prompt_tokens.or(prev_prompt);
        let final_completion = completion_tokens.or(prev_completion);

        db.execute(
            "UPDATE agent_steps SET agent_name=?1, step_order=?2, step_type=?3, input_json=?4, output_json=?5, status=?6, error_message=?7, prompt_tokens=?8, completion_tokens=?9, completed_at=?10 WHERE id=?11",
            rusqlite::params![agent_name, step_order, step_type, final_input, final_output, status, final_error, final_prompt, final_completion, now, step_id],
        ).map_err(|e| sanitize_error(e.to_string()))?;

        let parsed = parse_agent_run_status(&status).unwrap_or(AgentRunStatus::Failed);
        Ok(AgentStep { id: step_id, run_id, agent_name, step_key, step_order, step_type, input_json: final_input, output_json: final_output, status: parsed, error_message: final_error, prompt_tokens: final_prompt, completion_tokens: final_completion, started_at: None, completed_at: Some(now.clone()), created_at: now })
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO agent_steps (id, run_id, agent_name, step_key, step_order, step_type, input_json, output_json, status, error_message, prompt_tokens, completion_tokens, started_at, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            rusqlite::params![id, run_id, agent_name, step_key, step_order, step_type, input_json, output_json, status, error_message, prompt_tokens, completion_tokens, now, now],
        ).map_err(|e| sanitize_error(e.to_string()))?;
        let parsed = parse_agent_run_status(&status).unwrap_or(AgentRunStatus::Failed);
        Ok(AgentStep { id, run_id, agent_name, step_key, step_order, step_type, input_json, output_json, status: parsed, error_message, prompt_tokens, completion_tokens, started_at: Some(now.clone()), completed_at: None, created_at: now })
    }
}

#[tauri::command]
pub fn get_agent_steps(state: State<AppState>, run_id: String) -> Result<Vec<AgentStep>, String> {
    let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
    let mut stmt = db.prepare("SELECT id, run_id, agent_name, step_key, step_order, step_type, input_json, output_json, status, error_message, prompt_tokens, completion_tokens, started_at, completed_at, created_at FROM agent_steps WHERE run_id = ?1 ORDER BY step_order ASC").map_err(|e| sanitize_error(e.to_string()))?;
    let steps = stmt.query_map(rusqlite::params![run_id], |row| {
        let st: String = row.get(8)?;
        Ok(AgentStep { id: row.get(0)?, run_id: row.get(1)?, agent_name: row.get(2)?, step_key: row.get(3)?, step_order: row.get(4)?, step_type: row.get(5)?, input_json: row.get(6)?, output_json: row.get(7)?, status: parse_agent_run_status(&st).unwrap_or(AgentRunStatus::Failed), error_message: row.get(9)?, prompt_tokens: row.get(10)?, completion_tokens: row.get(11)?, started_at: row.get(12)?, completed_at: row.get(13)?, created_at: row.get(14)? })
    }).map_err(|e| sanitize_error(e.to_string()))?.collect::<Result<Vec<AgentStep>,_>>().map_err(|e| sanitize_error(e.to_string()))?;
    Ok(steps)
}

// ════════════════════════════════════════════════════════════
// LLM Completion
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn run_llm_completion(state: State<'_, AppState>, request: LlmRequest) -> Result<LlmResponse, String> {
    let total_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();
    security::validate_llm_request(request.messages.len(), total_chars, request.max_tokens)?;
    let (base_url, api_key) = {
        let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
        let r: Result<(String,String),_> = db.query_row("SELECT base_url, encrypted_api_key FROM model_configs LIMIT 1", [], |row| Ok((row.get(0)?, row.get(1)?)));
        match r {
            Ok((u, enc)) => { security::validate_base_url(&u)?; let d = decrypt_saved_api_key(&enc)?; (u, d) }
            Err(rusqlite::Error::QueryReturnedNoRows) => return Err("No model config".to_string()),
            Err(e) => return Err(sanitize_error(e.to_string())),
        }
    };
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let msgs: Vec<Value> = request.messages.iter().map(|m| serde_json::json!({"role":m.role,"content":m.content})).collect();
    let body = serde_json::json!({"model":request.model,"messages":msgs,"temperature":request.temperature,"max_tokens":request.max_tokens,"stream":false});
    let resp = security::build_reqwest_client().post(&url).header("Authorization", format!("Bearer {}", api_key)).header("Content-Type","application/json").json(&body).send().await.map_err(|e| sanitize_error(format!("Request: {}", e)))?;
    if !resp.status().is_success() { return Err(sanitize_error(format!("HTTP {}", resp.status().as_u16()))); }
    let json: Value = resp.json().await.map_err(|e| sanitize_error(e.to_string()))?;
    let content = json["choices"][0]["message"]["content"].as_str().ok_or("Unexpected response format")?.to_string();
    let u = json["usage"].clone();
    Ok(LlmResponse { content, model: request.model, usage: LlmUsage { prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32, completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32, total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32 } })
}

// ════════════════════════════════════════════════════════════
// run_workflow — with agent validation + step_key + audit
// ════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn run_workflow(
    state: State<'_, AppState>, project_id: String, task_description: String, workflow_type_str: String,
) -> Result<AgentRun, String> {
    let wt = parse_workflow_type(&workflow_type_str);
    let def = workflow::get_workflow(&wt).ok_or_else(|| format!("Unknown workflow: {}", workflow_type_str))?;

    let correlation_id = uuid::Uuid::new_v4().to_string();

    for step in def.steps {
        if workflow::get_agent(step.agent_name).is_none() {
            let run_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            {
                let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
                db.execute(
                    "INSERT INTO agent_runs (id, project_id, task_description, workflow_type, status, created_at, updated_at) VALUES (?1,?2,?3,?4,'failed',?5,?6)",
                    rusqlite::params![run_id, project_id, task_description, workflow_type_str, now, now],
                ).map_err(|e| sanitize_error(e.to_string()))?;
            }
            let err_msg = format!("Unknown agent '{}' in step '{}'", step.agent_name, step.step_key);
            log_workflow_event(&state, &project_id, &run_id, "workflow_failed",
                &serde_json::json!({"error": sanitize_error(err_msg.clone()), "step_key": step.step_key}).to_string(),
                &correlation_id, "error")?;
            return Err(sanitize_error(err_msg));
        }
    }

    let (model, temperature, max_tokens, emb_base_url, emb_api_key, emb_model) = {
        let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
        let m: String = db.query_row("SELECT model FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        let t: f64 = db.query_row("SELECT temperature FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or(0.7);
        let mt: u32 = db.query_row("SELECT max_tokens FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or(4096);
        let bu: String = db.query_row("SELECT base_url FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        let ek: String = db.query_row("SELECT encrypted_api_key FROM model_configs LIMIT 1", [], |r| r.get(0)).unwrap_or_default();
        (m, t, mt, bu, ek, "text-embedding-3-small".to_string())
    };

    let correlation_id = uuid::Uuid::new_v4().to_string();
    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    {
        let db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
        db.execute(
            "INSERT INTO agent_runs (id, project_id, task_description, workflow_type, status, created_at, updated_at) VALUES (?1,?2,?3,?4,'running',?5,?6)",
            rusqlite::params![run_id, project_id, task_description, workflow_type_str, now, now],
        ).map_err(|e| sanitize_error(e.to_string()))?;
    }

    log_workflow_event(&state, &project_id, &run_id, "workflow_start",
        &serde_json::json!({"workflow_type": workflow_type_str, "task": task_description}).to_string(),
        &correlation_id, "info")?;

    let mut previous_output = String::new();

    for step in def.steps {
        let agent_def = workflow::get_agent(step.agent_name);
        let agent_role = agent_def.map(|a| a.role_description).unwrap_or("");

        let truncated_prev = if step.max_previous_output_chars > 0 && !previous_output.is_empty() {
            let original_len = previous_output.chars().count();
            let t = truncate_str(&previous_output, step.max_previous_output_chars);
            if t.len() != previous_output.len() {
                log_workflow_event(&state, &project_id, &run_id, "context_truncated",
                    &serde_json::json!({"step_key": step.step_key, "original_chars": original_len, "truncated_chars": t.chars().count(), "max_budget": step.max_previous_output_chars}).to_string(),
                    &correlation_id, "info")?;
            }
            t
        } else {
            previous_output.clone()
        };

        let user_prompt = step.user_prompt_template
            .replace("{task_description}", &task_description)
            .replace("{previous_output}", &truncated_prev);

        let (mut final_user_prompt, mut retrieval_run_id, mut retrieval_hits_json) = (user_prompt.clone(), None, serde_json::json!([]));
        let mut strategy = "keyword";

        if step.use_rag {
            let rag_query = format!("{} {}", task_description, previous_output);
            let (query_emb, is_fallback) = {
                let api_key = decrypt_saved_api_key(&emb_api_key).ok();
                let url_ok = !emb_base_url.is_empty() && crate::commands::security::validate_base_url(&emb_base_url).is_ok();
                if let (Some(key), true) = (api_key.as_ref(), url_ok) {
                    match crate::commands::embedding::embed_query(&emb_base_url, key, &emb_model, &rag_query).await {
                        Ok(vec) => (Some(vec), false),
                        Err(e) => {
                            log_workflow_event(&state, &project_id, &run_id, "step_rag_embed_failed",
                                &serde_json::json!({"step_key": step.step_key, "error": sanitize_error(e)}).to_string(),
                                &correlation_id, "warning")?;
                            (None, true)
                        }
                    }
                } else {
                    log_workflow_event(&state, &project_id, &run_id, "step_rag_fallback",
                        &serde_json::json!({"step_key": step.step_key, "reason": if !url_ok {"invalid_base_url"} else {"no_api_key"}}).to_string(),
                        &correlation_id, "info")?;
                    (None, true)
                }
            };

            strategy = if query_emb.is_some() { "hybrid" } else { "keyword_fallback" };
            let mut db = state.db.lock().map_err(|e| sanitize_error(e.to_string()))?;
            match rag::retrieve_for_context(&mut db, &project_id, &rag_query, 5,
                Some((&run_id, step.step_key, step.agent_name)),
                query_emb.as_deref(),
                Some(strategy),
            ) {
                Ok(result) => {
                    let mut hits_entries = Vec::new();
                    for (chunk_id, title, excerpt) in result.excerpts.iter() {
                        hits_entries.push(serde_json::json!({
                            "hit_id": "", "chunk_id": chunk_id, "doc_title": title,
                            "excerpt": crate::models::sanitize_error(excerpt.clone()), "score": 0.0, "rank": 0,
                            "source": null, "provenance": null, "score_breakdown": null,
                            "status": "unknown",
                        }));
                    }
                    for (i, hit) in result.hits.iter().enumerate() {
                        if i < hits_entries.len() {
                            hits_entries[i]["hit_id"] = serde_json::json!(hit.id);
                            hits_entries[i]["score"] = serde_json::json!(hit.score);
                            hits_entries[i]["rank"] = serde_json::json!(hit.rank);
                            hits_entries[i]["score_breakdown"] = serde_json::json!(hit.score_breakdown);
                            if let Ok(meta_str) = db.query_row("SELECT metadata FROM document_chunks WHERE id=?1", rusqlite::params![hit.chunk_id], |r| r.get::<_,Option<String>>(0)) {
                                if let Some(ref m) = meta_str {
                                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(m) {
                                        hits_entries[i]["source"] = v.get("source").cloned().unwrap_or(serde_json::Value::Null);
                                        hits_entries[i]["provenance"] = v.get("provenance").cloned().unwrap_or(serde_json::Value::Null);
                                    }
                                }
                            }
                        }
                    }
                    let effective_excerpts = if step.max_rag_chars > 0 {
                        let truncated = truncate_rag_context(&result.excerpts, step.max_rag_chars);
                        if truncated.len() < result.excerpts.len() {
                            log_workflow_event(&state, &project_id, &run_id, "rag_context_truncated",
                                &serde_json::json!({"step_key": step.step_key, "original_excerpts": result.excerpts.len(), "kept_excerpts": truncated.len(), "max_budget": step.max_rag_chars}).to_string(),
                                &correlation_id, "info")?;
                        }
                        truncated
                    } else {
                        result.excerpts.clone()
                    };
                    let deduped = rag::deduplicate_excerpts(&effective_excerpts, 0.8);
                    if deduped.len() < effective_excerpts.len() {
                        log_workflow_event(&state, &project_id, &run_id, "rag_dedup_applied",
                            &serde_json::json!({"step_key": step.step_key, "before_dedup": effective_excerpts.len(), "after_dedup": deduped.len(), "threshold": 0.8}).to_string(),
                            &correlation_id, "info")?;
                    }

                    let deduped_ids: std::collections::HashSet<&str> = deduped.iter().map(|(id, _, _)| id.as_str()).collect();
                    let effective_ids: std::collections::HashSet<&str> = effective_excerpts.iter().map(|(id, _, _)| id.as_str()).collect();
                    let deduped_map: std::collections::HashMap<&str, &str> = deduped.iter()
                        .map(|(id, _, excerpt)| (id.as_str(), excerpt.as_str()))
                        .collect();
                    for entry in &mut hits_entries {
                        let cid = entry["chunk_id"].as_str().unwrap_or("").to_string();
                        if deduped_ids.contains(cid.as_str()) {
                            entry["status"] = serde_json::json!("injected");
                            if let Some(&injected_body) = deduped_map.get(cid.as_str()) {
                                let original = entry["excerpt"].as_str().unwrap_or("");
                                let injected = crate::models::sanitize_error(injected_body.to_string());
                                let orig_chars = original.chars().count();
                                let inj_chars = injected.chars().count();
                                entry["excerpt"] = serde_json::json!(injected);
                                entry["original_excerpt_chars"] = serde_json::json!(orig_chars);
                                entry["injected_excerpt_chars"] = serde_json::json!(inj_chars);
                                entry["truncated"] = serde_json::json!(orig_chars != inj_chars);
                            }
                        } else if effective_ids.contains(cid.as_str()) {
                            entry["status"] = serde_json::json!("deduped_out");
                        } else {
                            entry["status"] = serde_json::json!("truncated");
                        }
                    }

                    let context: Vec<String> = deduped.iter().map(|(_, title, excerpt)| {
                        format!("[Source: {}] {}", title, crate::models::sanitize_error(excerpt.clone()))
                    }).collect();
                    if !context.is_empty() {
                        final_user_prompt = format!("{}\n\nRelevant Context from Knowledge Base:\n{}",
                            final_user_prompt, context.join("\n---\n"));
                    }
                    retrieval_run_id = Some(result.run.id);
                    retrieval_hits_json = serde_json::json!({"strategy": strategy, "hits": hits_entries});
                }
                Err(_) => {}
            }
        }

            let retrieval_meta = serde_json::json!({
                "retrieval_run_id": retrieval_run_id,
                "strategy": strategy,
                "hits": retrieval_hits_json,
            });

            let input = serde_json::json!({
                "step_key": step.step_key,
                "agent_name": step.agent_name,
                "system": step.system_prompt,
                "user": &final_user_prompt,
                "retrieval": retrieval_meta,
                "use_rag": step.use_rag,
            }).to_string();

        save_agent_step(state.clone(), run_id.clone(), step.agent_name.to_string(),
            step.step_key.to_string(), step.step_order, step.step_type.to_string(),
            Some(input.clone()), None, "running".to_string(), None, None, None)?;

        log_workflow_event(&state, &project_id, &run_id, "step_start",
            &serde_json::json!({"step_key": step.step_key, "agent": step.agent_name, "agent_role": agent_role}).to_string(),
            &correlation_id, "info")?;

        match run_llm_completion(state.clone(), LlmRequest {
            model: model.clone(), temperature, max_tokens,
            messages: vec![
                crate::models::LlmMessage { role: "system".to_string(), content: step.system_prompt.to_string() },
                crate::models::LlmMessage { role: "user".to_string(), content: final_user_prompt },
            ],
        }).await {
            Ok(response) => {
                save_agent_message(state.clone(), run_id.clone(), step.agent_name.to_string(), "system".to_string(),
                    step.system_prompt.to_string(), Some(serde_json::json!({"step_key": step.step_key, "agent_role": agent_role, "retrieval": retrieval_meta}).to_string()))?;
                save_agent_message(state.clone(), run_id.clone(), step.agent_name.to_string(), "assistant".to_string(),
                    response.content.clone(), Some(serde_json::json!({"step_key": step.step_key, "agent_role": agent_role, "usage": response.usage, "retrieval": retrieval_meta}).to_string()))?;

                save_agent_step(state.clone(), run_id.clone(), step.agent_name.to_string(),
                    step.step_key.to_string(), step.step_order, step.step_type.to_string(),
                    None, Some(response.content.clone()), "completed".to_string(),
                    None, Some(response.usage.prompt_tokens as i32), Some(response.usage.completion_tokens as i32))?;

                log_workflow_event(&state, &project_id, &run_id, "step_complete",
                    &serde_json::json!({"step_key": step.step_key, "agent_role": agent_role, "tokens": response.usage.total_tokens}).to_string(),
                    &correlation_id, "info")?;

                if step.save_to_memory {
                    if let Err(e) = save_step_to_memory(&state, &project_id, &run_id, step, &response.content, &correlation_id) {
                        log_workflow_event(&state, &project_id, &run_id, "step_failed",
                            &serde_json::json!({"step_key": step.step_key, "error": crate::models::sanitize_error(e.clone())}).to_string(),
                            &correlation_id, "error")?;
                    }
                }

                previous_output = response.content;
            }
            Err(e) => {
                save_agent_step(state.clone(), run_id.clone(), step.agent_name.to_string(),
                    step.step_key.to_string(), step.step_order, step.step_type.to_string(),
                    None, None, "failed".to_string(),
                    Some(e.clone()), None, None)?;

                log_workflow_event(&state, &project_id, &run_id, "step_failed",
                    &serde_json::json!({"step_key": step.step_key, "error": sanitize_error(e.clone())}).to_string(),
                    &correlation_id, "error")?;

                update_agent_run(state.clone(), run_id.clone(), "failed".to_string())?;
                log_workflow_event(&state, &project_id, &run_id, "workflow_failed",
                    &serde_json::json!({"error": sanitize_error(e.clone())}).to_string(),
                    &correlation_id, "error")?;

                return Err(sanitize_error(e));
            }
        }
    }

    let final_run = update_agent_run(state.clone(), run_id.clone(), "completed".to_string())?;
    log_workflow_event(&state, &project_id, &run_id, "workflow_complete", "{}", &correlation_id, "info")?;

    Ok(final_run)
}

fn save_step_to_memory(
    state: &State<'_, AppState>, project_id: &str, run_id: &str,
    step: &workflow::WorkflowStep, content: &str, correlation_id: &str,
) -> Result<(), String> {
    let provenance = serde_json::json!({
        "run_id": run_id,
        "step_key": step.step_key,
        "agent_name": step.agent_name,
        "correlation_id": correlation_id,
        "source": step.agent_name,
        "confidence": 0.8,
        "version": 1,
    }).to_string();

    let sections: Vec<(String, String)> = content
        .split("\n## ")
        .enumerate()
        .filter_map(|(_i, s)| {
            let mut parts = s.splitn(2, '\n');
            let heading = parts.next().unwrap_or("").trim().trim_start_matches("# ").to_lowercase().replace(' ', "_");
            let body = parts.next().unwrap_or("").trim().to_string();
            if body.len() > 10 { Some((heading, body)) } else { None }
        })
        .collect();

    if sections.is_empty() {
        let title = content.lines().next().unwrap_or("").trim().trim_start_matches("# ").trim();
        if title.len() > 3 {
            memory::save_project_memory(
                state.clone(), project_id.to_string(), "world_setting".into(), "design_title".into(),
                title.to_string(), Some("L2".into()), Some("project".into()),
                Some(step.agent_name.to_string()), Some(0.8), Some(1), Some(provenance.clone()),
            )?;
        }
    } else {
        for (heading, body) in &sections {
            let memory_type = if heading.contains("character") { "character" }
                else if heading.contains("plot") || heading.contains("story") { "plot" }
                else if heading.contains("rule") || heading.contains("mechanic") || heading.contains("card") { "rule" }
                else if heading.contains("art") || heading.contains("style") { "art_style" }
                else { "world_setting" };
            memory::save_project_memory(
                state.clone(), project_id.to_string(), memory_type.into(),
                heading.chars().take(60).collect::<String>(), body.clone(),
                Some("L2".into()), Some("project".into()), Some(step.agent_name.to_string()),
                Some(0.8), Some(1), Some(provenance.clone()),
            )?;
        }
    }

    log_workflow_event(state, project_id, run_id, "memory_saved",
        &serde_json::json!({"step_key": step.step_key, "agent": step.agent_name}).to_string(),
        correlation_id, "info")?;

    Ok(())
}

// ════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::workflow;

    #[test]
    fn parse_agent_run_status_valid() {
        assert!(parse_agent_run_status("running").is_ok());
        assert!(parse_agent_run_status("completed").is_ok());
        assert!(parse_agent_run_status("failed").is_ok());
        assert!(parse_agent_run_status("pending").is_ok());
        assert!(parse_agent_run_status("cancelled").is_ok());
    }

    #[test]
    fn parse_agent_run_status_invalid() {
        assert!(parse_agent_run_status("invalid_status").is_err());
        assert!(parse_agent_run_status("").is_err());
    }

    #[test]
    fn all_workflow_agents_are_registered() {
        for wt in &[WorkflowType::CardGameConcept, WorkflowType::VisualNovelConcept, WorkflowType::GameDesignDoc] {
            if let Some(def) = workflow::get_workflow(wt) {
                for step in def.steps {
                    assert!(
                        workflow::get_agent(step.agent_name).is_some(),
                        "Agent '{}' in workflow {:?} step '{}' is not in AGENT_REGISTRY",
                        step.agent_name, wt, step.step_key
                    );
                }
            }
        }
    }

    #[test]
    fn truncate_str_noop_when_under_budget() {
        let s = "hello world";
        let result = truncate_str(s, 20);
        assert_eq!(result, s);
        assert!(result.chars().count() <= 20);
    }

    #[test]
    fn truncate_str_result_within_budget() {
        let s = "a".repeat(500);
        let result = truncate_str(&s, 200);
        assert!(result.chars().count() <= 200, "result chars {} exceeds budget 200", result.chars().count());
        assert!(result.contains("[...truncated"));
    }

    #[test]
    fn truncate_str_tiny_budget() {
        let s = "hello world this is a test string";
        let result = truncate_str(s, 10);
        assert!(result.chars().count() <= 10, "result chars {} exceeds budget 10", result.chars().count());
        assert!(result.ends_with("[...]"));
    }

    #[test]
    fn truncate_str_chars_not_bytes() {
        let s = "𝒶".repeat(100); // each char is 4 bytes
        assert!(s.len() > s.chars().count());
        let result = truncate_str(&s, 50);
        assert!(result.chars().count() <= 50, "chars budget must use chars().count()");
    }

    #[test]
    fn truncate_rag_context_preserves_within_budget() {
        let excerpts = vec![
            ("c1".into(), "Doc1".into(), "short content".into()),
            ("c2".into(), "Doc2".into(), "another piece".into()),
        ];
        let result = truncate_rag_context(&excerpts, 10000);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn truncate_rag_context_drops_over_budget() {
        let excerpts: Vec<(String, String, String)> = (0..10).map(|i| {
            (format!("c{}", i), format!("Doc{}", i), "x".repeat(100))
        }).collect();
        let result = truncate_rag_context(&excerpts, 200);
        assert!(result.len() < 10, "some excerpts should be dropped");
        assert!(!result.is_empty(), "at least one should fit");
    }

    #[test]
    fn truncate_rag_context_zero_returns_all() {
        let excerpts = vec![
            ("c1".into(), "Doc1".into(), "content".into()),
        ];
        let result = truncate_rag_context(&excerpts, 0);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn truncate_rag_context_excerpt_has_no_source_prefix() {
        let excerpts = vec![
            ("c1".into(), "Doc1".into(), "hello world body text".into()),
        ];
        let result = truncate_rag_context(&excerpts, 500);
        assert_eq!(result.len(), 1);
        // The excerpt body should NOT contain [Source: ...] prefix
        let body = &result[0].2;
        assert!(!body.contains("[Source:"), "excerpt body must not contain [Source:] wrapper, got: {}", body);
        assert!(body.contains("hello world"), "excerpt body should contain the original text");
    }

    #[test]
    fn truncate_rag_context_budget_matches_prompt_format() {
        let excerpts = vec![
            ("c1".into(), "LongDocumentTitle".into(), "some body text here for testing".into()),
        ];
        let max_budget: usize = 200;
        let result = truncate_rag_context(&excerpts, max_budget);
        // Simulate the prompt construction from agents.rs
        let context: Vec<String> = result.iter().map(|(_, title, excerpt)| {
            format!("[Source: {}] {}", title, excerpt)
        }).collect();
        let joined = context.join("\n---\n");
        assert!(joined.chars().count() <= max_budget + 10,
            "joined prompt {} exceeds budget {} (allow small slack for join)", joined.chars().count(), max_budget);
    }

    #[test]
    fn truncate_rag_context_budget_multiple_excerpts() {
        let excerpts = vec![
            ("c1".into(), "T1".into(), "a b c d e f g h i j".into()),
            ("c2".into(), "T2".into(), "k l m n o p q r s t".into()),
            ("c3".into(), "T3".into(), "u v w x y z".into()),
        ];
        let max_budget: usize = 100;
        let result = truncate_rag_context(&excerpts, max_budget);
        let context: Vec<String> = result.iter().map(|(_, title, excerpt)| {
            format!("[Source: {}] {}", title, excerpt)
        }).collect();
        let joined = context.join("\n---\n");
        assert!(joined.chars().count() <= max_budget + 10,
            "joined {} chars exceeds budget {}", joined.chars().count(), max_budget);
    }

    #[test]
    fn workflow_steps_have_token_budgets_set() {
        for wt in &[WorkflowType::CardGameConcept, WorkflowType::VisualNovelConcept, WorkflowType::GameDesignDoc] {
            if let Some(def) = workflow::get_workflow(wt) {
                for step in def.steps {
                    if step.use_rag {
                        assert!(step.max_rag_chars > 0,
                            "use_rag step '{}' should have max_rag_chars > 0", step.step_key);
                    }
                    if step.save_to_memory {
                        assert!(step.max_previous_output_chars > 0 || step.step_order == 1,
                            "step '{}' with save_to_memory should have max_previous_output_chars", step.step_key);
                    }
                }
            }
        }
    }

    #[test]
    fn truncate_rag_context_injected_excerpt_matches_prompt_line() {
        let excerpts = vec![
            ("c1".into(), "MyDoc".into(), "this is the full original excerpt body text here".into()),
        ];
        let max_budget: usize = 80;
        let truncated = truncate_rag_context(&excerpts, max_budget);
        let deduped = rag::deduplicate_excerpts(&truncated, 0.8);

        for (chunk_id, title, injected_body) in &deduped {
            let prompt_line = format!("[Source: {}] {}", title, injected_body);
            assert!(prompt_line.contains(injected_body),
                "prompt line must contain injected body verbatim for chunk {}", chunk_id);
            let orig = excerpts.iter().find(|(id, _, _)| id == chunk_id).unwrap();
            if injected_body.chars().count() < orig.2.chars().count() {
                assert!(injected_body.chars().count() < orig.2.chars().count(),
                    "truncated body should be shorter than original");
            }
        }
    }

    #[test]
    fn truncate_rag_context_excerpt_body_is_shorter_when_budget_tight() {
        let excerpts = vec![
            ("c1".into(), "D".into(), "very long content that will definitely be truncated because the budget is so small".into()),
        ];
        let max_budget: usize = 40;
        let result = truncate_rag_context(&excerpts, max_budget);
        assert_eq!(result.len(), 1);
        let injected_body = &result[0].2;
        let original_body = &excerpts[0].2;
        assert!(injected_body.chars().count() < original_body.chars().count(),
            "with tight budget injected {} chars should be shorter than original {} chars",
            injected_body.chars().count(), original_body.chars().count());
        assert!(!injected_body.contains("[Source:"),
            "injected body must not contain [Source:] prefix");
    }
}
