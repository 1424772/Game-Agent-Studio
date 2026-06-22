use crate::crypto::SecretStore;
use crate::models::{
    AgentMessage, AgentRun, AgentRunStatus, AgentStep, LlmRequest, LlmResponse, LlmUsage,
    MessageStatus, WorkflowType,
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

fn workflow_type_to_str(wt: &WorkflowType) -> String {
    match wt {
        WorkflowType::CardGameConcept => "card_game_concept".to_string(),
        WorkflowType::VisualNovelConcept => "visual_novel_concept".to_string(),
        WorkflowType::GameDesignDoc => "game_design_doc".to_string(),
        WorkflowType::Custom(s) => s.clone(),
    }
}

fn parse_agent_run_status(s: &str) -> AgentRunStatus {
    match s {
        "pending" => AgentRunStatus::Pending,
        "running" => AgentRunStatus::Running,
        "waiting_for_input" => AgentRunStatus::WaitingForInput,
        "completed" => AgentRunStatus::Completed,
        "failed" => AgentRunStatus::Failed,
        "cancelled" => AgentRunStatus::Cancelled,
        _ => AgentRunStatus::Pending,
    }
}

fn parse_message_status(s: &str) -> MessageStatus {
    match s {
        "pending" => MessageStatus::Pending,
        "streaming" => MessageStatus::Streaming,
        "completed" => MessageStatus::Completed,
        "failed" => MessageStatus::Failed,
        "cancelled" => MessageStatus::Cancelled,
        "accepted" => MessageStatus::Accepted,
        "rejected" => MessageStatus::Rejected,
        "edited" => MessageStatus::Edited,
        _ => MessageStatus::Pending,
    }
}

#[tauri::command]
pub fn create_agent_run(
    state: State<AppState>,
    project_id: String,
    task_description: String,
    workflow_type: String,
) -> Result<AgentRun, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let wt = parse_workflow_type(&workflow_type);
    let wt_str = workflow_type_to_str(&wt);

    db.execute(
        "INSERT INTO agent_runs (id, project_id, task_description, workflow_type, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)",
        rusqlite::params![id, project_id, task_description, wt_str, now, now],
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(AgentRun {
        id,
        project_id,
        task_description,
        workflow_type: wt,
        status: AgentRunStatus::Pending,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn get_agent_runs(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<AgentRun>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db
        .prepare("SELECT id, project_id, task_description, workflow_type, status, created_at, updated_at FROM agent_runs WHERE project_id = ?1 ORDER BY created_at DESC")
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let runs = stmt
        .query_map(rusqlite::params![project_id], |row| {
            let wt: String = row.get(3)?;
            let st: String = row.get(4)?;
            Ok(AgentRun {
                id: row.get(0)?,
                project_id: row.get(1)?,
                task_description: row.get(2)?,
                workflow_type: parse_workflow_type(&wt),
                status: parse_agent_run_status(&st),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .collect::<Result<Vec<AgentRun>, _>>()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(runs)
}

#[tauri::command]
pub fn get_agent_messages(
    state: State<AppState>,
    run_id: String,
) -> Result<Vec<AgentMessage>, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let mut stmt = db
        .prepare("SELECT id, run_id, agent_name, role, content, metadata, status, created_at FROM agent_messages WHERE run_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let messages = stmt
        .query_map(rusqlite::params![run_id], |row| {
            let st: String = row.get(6)?;
            let meta_raw: String = row.get(5)?;
            let metadata = if meta_raw.is_empty() || meta_raw == "{}" {
                None
            } else {
                Some(meta_raw)
            };
            Ok(AgentMessage {
                id: row.get(0)?,
                run_id: row.get(1)?,
                agent_name: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                metadata,
                status: parse_message_status(&st),
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?
        .collect::<Result<Vec<AgentMessage>, _>>()
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(messages)
}

#[tauri::command]
pub fn save_agent_message(
    state: State<AppState>,
    run_id: String,
    agent_name: String,
    role: String,
    content: String,
    metadata: Option<String>,
) -> Result<AgentMessage, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let has_meta = metadata.is_some();
    let meta_str = metadata.unwrap_or_else(|| "{}".to_string());

    db.execute(
        "INSERT INTO agent_messages (id, run_id, agent_name, role, content, metadata, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)",
        rusqlite::params![id, run_id, agent_name, role, content, meta_str, now],
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let stored_meta = if has_meta { Some(meta_str) } else { None };

    Ok(AgentMessage {
        id,
        run_id,
        agent_name,
        role,
        content,
        metadata: stored_meta,
        status: MessageStatus::Pending,
        created_at: now,
    })
}

#[tauri::command]
pub fn update_message_status(
    state: State<AppState>,
    message_id: String,
    status: String,
) -> Result<AgentMessage, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let parsed_status = parse_message_status(&status);
    let status_str = match &parsed_status {
        MessageStatus::Pending => "pending",
        MessageStatus::Streaming => "streaming",
        MessageStatus::Completed => "completed",
        MessageStatus::Failed => "failed",
        MessageStatus::Cancelled => "cancelled",
        MessageStatus::Accepted => "accepted",
        MessageStatus::Rejected => "rejected",
        MessageStatus::Edited => "edited",
    };

    db.execute(
        "UPDATE agent_messages SET status = ?1 WHERE id = ?2",
        rusqlite::params![status_str, message_id],
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let message = db
        .query_row(
            "SELECT id, run_id, agent_name, role, content, metadata, status, created_at FROM agent_messages WHERE id = ?1",
            rusqlite::params![message_id],
            |row| {
                let st: String = row.get(6)?;
                let meta_raw: String = row.get(5)?;
                let metadata = if meta_raw.is_empty() || meta_raw == "{}" {
                    None
                } else {
                    Some(meta_raw)
                };
                Ok(AgentMessage {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    agent_name: row.get(2)?,
                    role: row.get(3)?,
                    content: row.get(4)?,
                    metadata,
                    status: parse_message_status(&st),
                    created_at: row.get(7)?,
                })
            },
        )
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(message)
}

#[tauri::command]
pub fn save_agent_step(
    state: State<AppState>,
    run_id: String,
    agent_name: String,
    step_order: i32,
    step_type: String,
    input_json: Option<String>,
    output_json: Option<String>,
    status: String,
    error_message: Option<String>,
    prompt_tokens: Option<i32>,
    completion_tokens: Option<i32>,
) -> Result<AgentStep, String> {
    let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let parsed_status = parse_agent_run_status(&status);

    db.execute(
        "INSERT INTO agent_steps (id, run_id, agent_name, step_order, step_type, input_json, output_json, status, error_message, prompt_tokens, completion_tokens, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            id,
            run_id,
            agent_name,
            step_order,
            step_type,
            input_json,
            output_json,
            status,
            error_message,
            prompt_tokens,
            completion_tokens,
            now,
        ],
    )
    .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    Ok(AgentStep {
        id,
        run_id,
        agent_name,
        step_order,
        step_type,
        input_json,
        output_json,
        status: parsed_status,
        error_message,
        prompt_tokens,
        completion_tokens,
        started_at: None,
        completed_at: None,
        created_at: now,
    })
}

#[tauri::command]
pub async fn run_llm_completion(
    state: State<'_, AppState>,
    request: LlmRequest,
) -> Result<LlmResponse, String> {
    let (base_url, api_key) = {
        let db = state.db.lock().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
        let result: Result<(String, String), rusqlite::Error> = db.query_row(
            "SELECT base_url, encrypted_api_key FROM model_configs LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        match result {
            Ok((url, encrypted)) => {
                let store = SecretStore::new();
                let decrypted = store.decrypt(&encrypted).map_err(|e| {
                    crate::models::sanitize_error(format!("Failed to decrypt API key: {}", e))
                })?;
                (url, decrypted)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err("No model configuration found. Please save a model config first.".to_string());
            }
            Err(e) => return Err(crate::models::sanitize_error(e.to_string())),
        }
    };

    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));

    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content
            })
        })
        .collect();

    let body = serde_json::json!({
        "model": request.model,
        "messages": messages,
        "temperature": request.temperature,
        "max_tokens": request.max_tokens,
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
        .map_err(|e| crate::models::sanitize_error(format!("Request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(crate::models::sanitize_error(format!(
            "HTTP error: {}",
            status
        )));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| crate::models::sanitize_error(e.to_string()))?;

    let content = json["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .ok_or_else(|| "Unexpected response format".to_string())?
        .to_string();

    let usage = json["usage"].clone();
    let usage_obj = LlmUsage {
        prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
        total_tokens: usage["total_tokens"].as_u64().unwrap_or(0) as u32,
    };

    Ok(LlmResponse {
        content,
        model: request.model,
        usage: usage_obj,
    })
}
