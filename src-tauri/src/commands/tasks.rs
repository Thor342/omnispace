use crate::models::Task;
use crate::AppState;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn get_tasks(space_id: String, state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, space_id, title, completed, order_index, created_at, updated_at FROM tasks WHERE space_id=?1 ORDER BY order_index ASC, created_at ASC")
        .map_err(|e| e.to_string())?;

    let tasks = stmt
        .query_map(rusqlite::params![space_id], |row| {
            Ok(Task {
                id: row.get(0)?,
                space_id: row.get(1)?,
                title: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                order_index: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tasks)
}

#[tauri::command]
pub async fn create_task(
    space_id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<Task, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    // Get max order_index for this space
    let max_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(order_index), -1) FROM tasks WHERE space_id=?1",
            rusqlite::params![space_id],
            |row| row.get(0),
        )
        .unwrap_or(-1);
    let order_index = max_order + 1;

    db.execute(
        "INSERT INTO tasks (id, space_id, title, completed, order_index, created_at, updated_at) VALUES (?1,?2,?3,0,?4,?5,?6)",
        rusqlite::params![id, space_id, title, order_index, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Task {
        id,
        space_id,
        title,
        completed: false,
        order_index,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub async fn toggle_task(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();

    let current: i32 = db
        .query_row("SELECT completed FROM tasks WHERE id=?1", rusqlite::params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let new_val = if current == 0 { 1 } else { 0 };
    db.execute(
        "UPDATE tasks SET completed=?1, updated_at=?2 WHERE id=?3",
        rusqlite::params![new_val, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(new_val != 0)
}

#[tauri::command]
pub async fn update_task_title(
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE tasks SET title=?1, updated_at=?2 WHERE id=?3",
        rusqlite::params![title, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM tasks WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
