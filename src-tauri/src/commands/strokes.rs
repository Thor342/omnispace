use crate::AppState;
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub async fn get_strokes(page_id: String, state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db.query_row(
        "SELECT data FROM page_strokes WHERE page_id=?1",
        rusqlite::params![page_id],
        |row| row.get::<_, String>(0),
    );
    Ok(result.unwrap_or_else(|_| "[]".to_string()))
}

#[tauri::command]
pub async fn save_strokes(
    page_id: String,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO page_strokes (page_id, data, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(page_id) DO UPDATE SET data=excluded.data, updated_at=excluded.updated_at",
        rusqlite::params![page_id, data, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
