use crate::models::Link;
use crate::AppState;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

fn detect_link_type(url: &str) -> String {
    if url.contains("youtube.com/watch") || url.contains("youtu.be/") {
        "youtube".to_string()
    } else {
        "general".to_string()
    }
}


#[tauri::command]
pub async fn get_links(space_id: String, state: State<'_, AppState>) -> Result<Vec<Link>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, space_id, title, url, link_type, created_at FROM links WHERE space_id=?1 ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let links = stmt
        .query_map(rusqlite::params![space_id], |row| {
            Ok(Link {
                id: row.get(0)?,
                space_id: row.get(1)?,
                title: row.get(2)?,
                url: row.get(3)?,
                link_type: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(links)
}

#[tauri::command]
pub async fn create_link(
    space_id: String,
    title: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Link, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let link_type = detect_link_type(&url);

    db.execute(
        "INSERT INTO links (id, space_id, title, url, link_type, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![id, space_id, title, url, link_type, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Link { id, space_id, title, url, link_type, created_at: now })
}

#[tauri::command]
pub async fn delete_link(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM links WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
