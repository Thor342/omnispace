use crate::models::{Page, Block};
use crate::AppState;
use chrono::Utc;
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

const MAX_TITLE_LEN: usize = 200;
const MAX_CONTENT_LEN: usize = 10 * 1024 * 1024; // 10 MB

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
    if title.len() > MAX_TITLE_LEN { return Err("Título demasiado largo".to_string()); }
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
    if title.len() > MAX_TITLE_LEN { return Err("Título demasiado largo".to_string()); }
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

// ── Export page as a compressed .omnipage zip ─────────────────────────────
#[tauri::command]
pub async fn export_page(
    page_id: String,
    dest_path: String,
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

    // Collect file data for blocks that reference files
    struct FileEntry { zip_name: String, data: Vec<u8> }
    let mut file_entries: Vec<FileEntry> = Vec::new();

    let export_blocks: Vec<serde_json::Value> = blocks.iter().map(|block| {
        let mut content_val: serde_json::Value =
            serde_json::from_str(&block.content).unwrap_or(serde_json::json!({}));

        if block.block_type == "file" {
            if let Some(stored_path) = content_val.get("stored_path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                if !stored_path.is_empty() {
                    let src = PathBuf::from(&stored_path);
                    if src.exists() {
                        if let Some(file_name) = src.file_name() {
                            let zip_name = format!("files/{}", file_name.to_string_lossy());
                            if let Ok(data) = std::fs::read(&src) {
                                file_entries.push(FileEntry { zip_name: zip_name.clone(), data });
                                content_val["stored_path"] = serde_json::json!(zip_name);
                            }
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

    // Build manifest JSON
    let manifest = serde_json::json!({
        "version": 1,
        "page": { "title": page.title },
        "blocks": export_blocks,
        "strokes": strokes,
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;

    // Write zip to dest_path
    let zip_file = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("page.json", options).map_err(|e| e.to_string())?;
    zip.write_all(&manifest_bytes).map_err(|e| e.to_string())?;

    for entry in file_entries {
        zip.start_file(&entry.zip_name, options).map_err(|e| e.to_string())?;
        zip.write_all(&entry.data).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;

    Ok(dest_path)
}

// ── Import page from a compressed .omnipage zip ───────────────────────────
#[tauri::command]
pub async fn import_page(
    space_id: String,
    import_path: String,
    state: State<'_, AppState>,
) -> Result<Page, String> {
    let zip_file = std::fs::File::open(&import_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(zip_file).map_err(|e| e.to_string())?;

    // Read page.json from zip
    let manifest: serde_json::Value = {
        let mut entry = archive.by_name("page.json").map_err(|_| "Archivo inválido: falta page.json".to_string())?;
        let mut buf = String::new();
        entry.read_to_string(&mut buf).map_err(|e| e.to_string())?;
        serde_json::from_str(&buf).map_err(|e| e.to_string())?
    };

    // Extract file entries into memory: zip_path -> bytes
    let mut file_map: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name.starts_with("files/") && !name.ends_with('/') {
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
            file_map.insert(name, data);
        }
    }

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
            if raw_content.len() > MAX_CONTENT_LEN { continue; }

            let content = if block_type == "file" {
                if let Ok(mut cv) = serde_json::from_str::<serde_json::Value>(raw_content) {
                    if let Some(rel) = cv.get("stored_path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                        if rel.starts_with("files/") {
                            if let Some(data) = file_map.get(&rel) {
                                let file_name = PathBuf::from(&rel)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "file".to_string());
                                let uid = &Uuid::new_v4().to_string()[..8];
                                let new_name = format!("{}_{}", uid, file_name);
                                let dest = files_dest.join(&new_name);
                                if std::fs::write(&dest, data).is_ok() {
                                    cv["stored_path"] = serde_json::json!(dest.to_string_lossy().to_string());
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
