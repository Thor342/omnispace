use crate::models::Block;
use crate::AppState;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

const MAX_CONTENT_LEN: usize = 10 * 1024 * 1024; // 10 MB

#[tauri::command]
pub async fn get_blocks(page_id: String, state: State<'_, AppState>) -> Result<Vec<Block>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, page_id, block_type, x, y, width, height, content, z_index, created_at, updated_at FROM blocks WHERE page_id=?1 ORDER BY z_index ASC")
        .map_err(|e| e.to_string())?;

    let blocks = stmt
        .query_map(rusqlite::params![page_id], |row| {
            Ok(Block {
                id: row.get(0)?,
                page_id: row.get(1)?,
                block_type: row.get(2)?,
                x: row.get(3)?,
                y: row.get(4)?,
                width: row.get(5)?,
                height: row.get(6)?,
                content: row.get(7)?,
                z_index: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(blocks)
}

#[tauri::command]
pub async fn create_block(
    page_id: String,
    block_type: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    content: String,
    state: State<'_, AppState>,
) -> Result<Block, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    let max_z: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(z_index), 0) FROM blocks WHERE page_id=?1",
            rusqlite::params![page_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let z_index = max_z + 1;

    db.execute(
        "INSERT INTO blocks (id, page_id, block_type, x, y, width, height, content, z_index, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params![id, page_id, block_type, x, y, width, height, content, z_index, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Block { id, page_id, block_type, x, y, width, height, content, z_index, created_at: now.clone(), updated_at: now })
}

#[tauri::command]
pub async fn update_block_position(
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE blocks SET x=?1, y=?2, width=?3, height=?4, updated_at=?5 WHERE id=?6",
        rusqlite::params![x, y, width, height, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_block_content(
    id: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if content.len() > MAX_CONTENT_LEN { return Err("Contenido demasiado grande".to_string()); }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE blocks SET content=?1, updated_at=?2 WHERE id=?3",
        rusqlite::params![content, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_block_zindex(
    id: String,
    z_index: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE blocks SET z_index=?1 WHERE id=?2",
        rusqlite::params![z_index, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_block(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM blocks WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn restore_block(block: Block, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR IGNORE INTO blocks (id, page_id, block_type, x, y, width, height, content, z_index, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params![block.id, block.page_id, block.block_type, block.x, block.y, block.width, block.height, block.content, block.z_index, block.created_at, block.updated_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
