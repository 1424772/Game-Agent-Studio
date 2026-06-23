mod commands;
mod crypto;
mod db;
mod models;

use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

/// Authoritative sorted list of all command names exposed via invoke_handler.
/// Must match generate_handler![] below exactly 鈥?verified by test.
pub const HANDLER_NAMES: &[&str] = &[
    "chunk_document",
    "create_agent_run",
    "create_document",
    "create_improvement_proposal",
    "create_project",
    "delete_project",
    "embed_pending_chunks",
    "export_json",
    "export_markdown",
    "get_agent_messages",
    "get_agent_run",
    "get_agent_runs",
    "get_agent_steps",
    "get_document_chunks",
    "get_events",
    "get_exports",
    "get_memory_versions",
    "get_model_config",
    "get_project",
    "get_project_memory",
    "get_retrieval_hit_excerpts",
    "get_retrieval_hits",
    "get_retrieval_runs",
    "get_user_preferences",
    "list_documents",
    "list_improvement_proposals",
    "list_projects",
    "log_event",
    "review_improvement_proposal",
    "run_llm_completion",
    "run_workflow",
    "save_agent_message",
    "save_agent_step",
    "save_model_config",
    "save_project_memory",
    "search_documents",
    "test_model_connection",
    "update_agent_message_content",
    "update_agent_run",
    "update_message_status",
    "update_user_preferences",
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = get_db_path();
    let conn = db::init::initialize_database(&db_path)
        .expect("Failed to initialize database");

    tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(conn),
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::create_project,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::delete_project,
            commands::model_configs::save_model_config,
            commands::model_configs::get_model_config,
            commands::model_configs::test_model_connection,
            commands::agents::create_agent_run,
            commands::agents::update_agent_run,
            commands::agents::get_agent_run,
            commands::agents::get_agent_runs,
            commands::agents::get_agent_messages,
            commands::agents::get_agent_steps,
            commands::agents::save_agent_message,
            commands::agents::update_message_status,
            commands::agents::update_agent_message_content,
            commands::agents::save_agent_step,
            commands::agents::run_llm_completion,
            commands::agents::run_workflow,
            commands::events::log_event,
            commands::events::get_events,
            commands::memory::get_project_memory,
            commands::memory::get_memory_versions,
            commands::memory::save_project_memory,
            commands::memory::get_user_preferences,
            commands::memory::update_user_preferences,
            commands::exports::export_markdown,
            commands::exports::export_json,
            commands::exports::get_exports,
            commands::iterations::create_improvement_proposal,
            commands::iterations::list_improvement_proposals,
            commands::iterations::review_improvement_proposal,
            commands::rag::create_document,
            commands::rag::chunk_document,
            commands::rag::list_documents,
            commands::rag::get_document_chunks,
            commands::rag::search_documents,
            commands::rag::get_retrieval_runs,
            commands::rag::get_retrieval_hits,
            commands::rag::get_retrieval_hit_excerpts,
            commands::rag::embed_pending_chunks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn get_db_path() -> String {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("game-agent-studio");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("app.db").to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_names_is_sorted() {
        let mut sorted = HANDLER_NAMES.to_vec();
        sorted.sort();
        assert_eq!(HANDLER_NAMES, sorted.as_slice(), "HANDLER_NAMES must be sorted");
    }

    #[test]
    fn handler_names_has_no_duplicates() {
        let mut deduped: Vec<&str> = HANDLER_NAMES.to_vec();
        deduped.dedup();
        assert_eq!(HANDLER_NAMES.len(), deduped.len(), "HANDLER_NAMES must have no duplicates");
    }

    #[test]
    fn handler_names_count_matches_generate_handler() {
        // generate_handler! has 41 entries. If you add/remove one, update this + HANDLER_NAMES.
        assert_eq!(HANDLER_NAMES.len(), 41, "HANDLER_NAMES count must match generate_handler![] entry count");
    }
}
