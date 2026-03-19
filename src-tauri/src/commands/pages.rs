use crate::models::{Page, Block};
use crate::AppState;
use chrono::Utc;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn get_pages(space_id: String, state: State<'_, AppState>) -> Result<Vec<Page>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, space_id, title, order_index, created_at, updated_at FROM pages WHERE space_id=?1 ORDER BY order_index ASC, created_at ASC")
        .map_err(|e| e.to_string())?;

    let pages = stmt
        .query_map(rusqlite::params![space_id], |row| {
            Ok(Page {
                id: row.get(0)?,
                space_id: row.get(1)?,
                title: row.get(2)?,
                order_index: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(pages)
}

#[tauri::command]
pub async fn create_page(
    space_id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<Page, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    let max_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(order_index), -1) FROM pages WHERE space_id=?1",
            rusqlite::params![space_id],
            |row| row.get(0),
        )
        .unwrap_or(-1);
    let order_index = max_order + 1;

    db.execute(
        "INSERT INTO pages (id, space_id, title, order_index, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![id, space_id, title, order_index, now, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Page { id, space_id, title, order_index, created_at: now.clone(), updated_at: now })
}

/// Recibe los IDs de páginas en el nuevo orden y actualiza order_index en BD
#[tauri::command]
pub async fn reorder_pages(
    page_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    for (idx, id) in page_ids.iter().enumerate() {
        db.execute(
            "UPDATE pages SET order_index=?1, updated_at=?2 WHERE id=?3",
            rusqlite::params![idx as i64, now, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn update_page_title(
    id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    db.execute(
        "UPDATE pages SET title=?1, updated_at=?2 WHERE id=?3",
        rusqlite::params![title, now, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_page(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM pages WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Export page as a folder ────────────────────────────────────────────────
#[tauri::command]
pub async fn export_page(
    page_id: String,
    dest_dir: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get page
    let page: Page = db
        .query_row(
            "SELECT id, space_id, title, order_index, created_at, updated_at FROM pages WHERE id=?1",
            rusqlite::params![page_id],
            |row| Ok(Page {
                id: row.get(0)?, space_id: row.get(1)?, title: row.get(2)?,
                order_index: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)?,
            }),
        )
        .map_err(|e| e.to_string())?;

    // Get blocks
    let mut stmt = db
        .prepare("SELECT id, page_id, block_type, x, y, width, height, content, z_index, created_at, updated_at FROM blocks WHERE page_id=?1 ORDER BY z_index ASC")
        .map_err(|e| e.to_string())?;
    let blocks: Vec<Block> = stmt
        .query_map(rusqlite::params![page_id], |row| {
            Ok(Block {
                id: row.get(0)?, page_id: row.get(1)?, block_type: row.get(2)?,
                x: row.get(3)?, y: row.get(4)?, width: row.get(5)?, height: row.get(6)?,
                content: row.get(7)?, z_index: row.get(8)?,
                created_at: row.get(9)?, updated_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get strokes
    let strokes: String = db
        .query_row("SELECT data FROM page_strokes WHERE page_id=?1", rusqlite::params![page_id], |row| row.get(0))
        .unwrap_or_else(|_| "[]".to_string());

    // Create export folder: dest_dir/<SafeTitle>_omnipage/
    let safe_title: String = page.title.chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
        .collect::<String>()
        .trim().to_string();
    let export_folder = PathBuf::from(&dest_dir).join(format!("{}_omnipage", safe_title));
    std::fs::create_dir_all(&export_folder).map_err(|e| e.to_string())?;
    let files_export_dir = export_folder.join("files");
    std::fs::create_dir_all(&files_export_dir).map_err(|e| e.to_string())?;

    // Build block list — for file blocks, copy the file and rewrite path to relative
    let export_blocks: Vec<serde_json::Value> = blocks.iter().map(|block| {
        let mut content_val: serde_json::Value =
            serde_json::from_str(&block.content).unwrap_or(serde_json::json!({}));

        if block.block_type == "file" {
            if let Some(stored_path) = content_val.get("stored_path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                if !stored_path.is_empty() {
                    let src = PathBuf::from(&stored_path);
                    if src.exists() {
                        if let Some(file_name) = src.file_name() {
                            let dest = files_export_dir.join(file_name);
                            let _ = std::fs::copy(&src, &dest);
                            content_val["stored_path"] = serde_json::json!(
                                format!("files/{}", file_name.to_string_lossy())
                            );
                        }
                    }
                }
            }
        }

        serde_json::json!({
            "block_type": block.block_type,
            "x": block.x, "y": block.y,
            "width": block.width, "height": block.height,
            "content": serde_json::to_string(&content_val).unwrap_or_else(|_| block.content.clone()),
            "z_index": block.z_index,
        })
    }).collect();

    // Write page.json
    let manifest = serde_json::json!({
        "version": 1,
        "page": { "title": page.title },
        "blocks": export_blocks,
        "strokes": strokes,
    });
    let json_path = export_folder.join("page.json");
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(export_folder.to_string_lossy().to_string())
}

// ── Import page from an exported folder ───────────────────────────────────
#[tauri::command]
pub async fn import_page(
    space_id: String,
    import_dir: String,
    state: State<'_, AppState>,
) -> Result<Page, String> {
    let import_path = PathBuf::from(&import_dir);
    let manifest_file = import_path.join("page.json");

    let manifest_str = std::fs::read_to_string(&manifest_file).map_err(|e| e.to_string())?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).map_err(|e| e.to_string())?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let page_id = Uuid::new_v4().to_string();

    let title = manifest["page"]["title"].as_str().unwrap_or("Página importada").to_string();

    let max_order: i64 = db
        .query_row("SELECT COALESCE(MAX(order_index), -1) FROM pages WHERE space_id=?1", rusqlite::params![space_id], |row| row.get(0))
        .unwrap_or(-1);

    db.execute(
        "INSERT INTO pages (id, space_id, title, order_index, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
        rusqlite::params![page_id, space_id, title, max_order + 1, now, now],
    ).map_err(|e| e.to_string())?;

    // Destination for imported files
    let files_dest = state.files_dir.join(&space_id);
    std::fs::create_dir_all(&files_dest).map_err(|e| e.to_string())?;

    // Recreate blocks
    if let Some(blocks) = manifest["blocks"].as_array() {
        for block in blocks {
            let block_id = Uuid::new_v4().to_string();
            let block_type = block["block_type"].as_str().unwrap_or("note");
            let x       = block["x"].as_f64().unwrap_or(100.0);
            let y       = block["y"].as_f64().unwrap_or(100.0);
            let width   = block["width"].as_f64().unwrap_or(420.0);
            let height  = block["height"].as_f64().unwrap_or(300.0);
            let z_index = block["z_index"].as_i64().unwrap_or(0);
            let raw_content = block["content"].as_str().unwrap_or("{}");

            // For file blocks: copy the file to app dir and update the stored_path
            let content = if block_type == "file" {
                if let Ok(mut cv) = serde_json::from_str::<serde_json::Value>(raw_content) {
                    if let Some(rel) = cv.get("stored_path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                        if rel.starts_with("files/") {
                            let src = import_path.join(&rel);
                            if src.exists() {
                                if let Some(file_name) = src.file_name() {
                                    let uid = &Uuid::new_v4().to_string()[..8];
                                    let new_name = format!("{}_{}", uid, file_name.to_string_lossy());
                                    let dest = files_dest.join(&new_name);
                                    if std::fs::copy(&src, &dest).is_ok() {
                                        cv["stored_path"] = serde_json::json!(dest.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                    serde_json::to_string(&cv).unwrap_or_else(|_| raw_content.to_string())
                } else {
                    raw_content.to_string()
                }
            } else {
                raw_content.to_string()
            };

            db.execute(
                "INSERT INTO blocks (id, page_id, block_type, x, y, width, height, content, z_index, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                rusqlite::params![block_id, page_id, block_type, x, y, width, height, content, z_index, now, now],
            ).map_err(|e| e.to_string())?;
        }
    }

    // Restore strokes
    let strokes = manifest["strokes"].as_str().unwrap_or("[]").to_string();
    db.execute(
        "INSERT OR REPLACE INTO page_strokes (page_id, data, updated_at) VALUES (?1,?2,?3)",
        rusqlite::params![page_id, strokes, now],
    ).map_err(|e| e.to_string())?;

    Ok(Page {
        id: page_id, space_id, title,
        order_index: max_order + 1,
        created_at: now.clone(), updated_at: now,
    })
}
