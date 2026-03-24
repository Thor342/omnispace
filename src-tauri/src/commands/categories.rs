use crate::models::Category;
use crate::AppState;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

fn validate_category_inputs(name: &str, icon: &str, color: &str) -> Result<(), String> {
    if name.trim().is_empty() { return Err("El nombre no puede estar vacío".to_string()); }
    if name.len() > 100 { return Err("El nombre no puede superar 100 caracteres".to_string()); }
    if icon.len() > 20 { return Err("Icono inválido".to_string()); }
    let valid_color = color.starts_with('#') && matches!(color.len(), 4 | 7) &&
        color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid_color { return Err("Color inválido, debe ser formato #RRGGBB".to_string()); }
    Ok(())
}

#[tauri::command]
pub async fn get_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, name, icon, color, order_index, created_at, updated_at FROM categories ORDER BY order_index ASC, created_at ASC")
        .map_err(|e| e.to_string())?;
    let cats = stmt
        .query_map([], |row| Ok(Category {
            id: row.get(0)?, name: row.get(1)?, icon: row.get(2)?, color: row.get(3)?,
            order_index: row.get(4)?, created_at: row.get(5)?, updated_at: row.get(6)?,
        }))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(cats)
}

#[tauri::command]
pub async fn create_category(name: String, icon: String, color: String, state: State<'_, AppState>) -> Result<Category, String> {
    validate_category_inputs(&name, &icon, &color)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let max_order: i64 = db
        .query_row("SELECT COALESCE(MAX(order_index), -1) FROM categories", [], |row| row.get(0))
        .unwrap_or(-1);
    db.execute(
        "INSERT INTO categories (id, name, icon, color, order_index, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![id, name, icon, color, max_order + 1, now, now],
    ).map_err(|e| e.to_string())?;
    Ok(Category { id, name, icon, color, order_index: max_order + 1, created_at: now.clone(), updated_at: now })
}

#[tauri::command]
pub async fn rename_category(id: String, name: String, icon: String, color: String, state: State<'_, AppState>) -> Result<(), String> {
    validate_category_inputs(&name, &icon, &color)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE categories SET name=?1, icon=?2, color=?3, updated_at=?4 WHERE id=?5",
        rusqlite::params![name, icon, color, now, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_category(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Unassign spaces from this category
    db.execute("UPDATE spaces SET category_id = NULL WHERE category_id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    db.execute("DELETE FROM categories WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn assign_space_to_category(space_id: String, category_id: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE spaces SET category_id=?1, updated_at=?2 WHERE id=?3",
        rusqlite::params![category_id, now, space_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
