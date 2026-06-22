use crate::models::{GameType, Project};
use crate::AppState;
use tauri::State;

fn parse_game_type(s: &str) -> GameType {
    match s {
        "card_game" => GameType::CardGame,
        "visual_novel" => GameType::VisualNovel,
        "rpg" => GameType::Rpg,
        "puzzle" => GameType::Puzzle,
        "strategy" => GameType::Strategy,
        "simulation" => GameType::Simulation,
        _ => GameType::Other(s.to_string()),
    }
}

fn game_type_to_str(gt: &GameType) -> String {
    match gt {
        GameType::CardGame => "card_game".to_string(),
        GameType::VisualNovel => "visual_novel".to_string(),
        GameType::Rpg => "rpg".to_string(),
        GameType::Puzzle => "puzzle".to_string(),
        GameType::Strategy => "strategy".to_string(),
        GameType::Simulation => "simulation".to_string(),
        GameType::Other(s) => s.clone(),
    }
}

#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    name: String,
    game_type: String,
    description: String,
) -> Result<Project, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let gt = parse_game_type(&game_type);
    let gt_str = game_type_to_str(&gt);

    db.execute(
        "INSERT INTO projects (id, name, game_type, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, name, gt_str, description, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Project {
        id,
        name,
        game_type: gt,
        description,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, name, game_type, description, created_at, updated_at FROM projects ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let projects = stmt
        .query_map([], |row| {
            let gt: String = row.get(2)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                game_type: parse_game_type(&gt),
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<Project>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(projects)
}

#[tauri::command]
pub fn get_project(state: State<AppState>, id: String) -> Result<Project, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.query_row(
        "SELECT id, name, game_type, description, created_at, updated_at FROM projects WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            let gt: String = row.get(2)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                game_type: parse_game_type(&gt),
                description: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}
