mod commands;
mod crypto;
mod db;
mod models;

use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

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
            commands::agents::get_agent_runs,
            commands::agents::get_agent_messages,
            commands::agents::save_agent_message,
            commands::agents::update_message_status,
            commands::agents::save_agent_step,
            commands::agents::run_llm_completion,
            commands::events::log_event,
            commands::events::get_events,
            commands::memory::get_project_memory,
            commands::memory::save_project_memory,
            commands::memory::get_user_preferences,
            commands::memory::update_user_preferences,
            commands::exports::export_markdown,
            commands::exports::export_json,
            commands::exports::get_exports,
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
