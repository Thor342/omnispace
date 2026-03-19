use crate::models::Space;
use crate::AppState;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn get_spaces(state: State<'_, AppState>) -> Result<Vec<Space>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, name, icon, color, category_id, created_at, updated_at FROM spaces ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let spaces = stmt
        .query_map([], |row| {
            Ok(Space {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                category_id: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(spaces)
}

#[tauri::command]
pub async fn create_space(
    name: String,
    icon: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<Space, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    db.execute(
        "INSERT INTO spaces (id, name, icon, color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, name, icon, color, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Space {
        id,
        name,
        icon,
        color,
        category_id: None,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_space(
    id: String,
    name: String,
    icon: String,
    color: String,
    state: State<'_, AppState>,
) -> Result<Space, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();

    db.execute(
        "UPDATE spaces SET name=?1, icon=?2, color=?3, updated_at=?4 WHERE id=?5",
        rusqlite::params![name, icon, color, now, id],
    )
    .map_err(|e| e.to_string())?;

    let created_at: String = db
        .query_row("SELECT created_at FROM spaces WHERE id=?1", rusqlite::params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    Ok(Space { id, name, icon, color, category_id: None, created_at, updated_at: now })
}

#[tauri::command]
pub async fn delete_space(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM spaces WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
