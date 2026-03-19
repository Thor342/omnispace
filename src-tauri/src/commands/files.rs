use crate::models::AppFile;
use crate::AppState;
use chrono::Utc;
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

fn detect_file_type(path: &str) -> String {
    let lower = path.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
        || lower.ends_with(".gif") || lower.ends_with(".webp") || lower.ends_with(".svg")
        || lower.ends_with(".bmp")
    {
        "image".to_string()
    } else if lower.ends_with(".pdf") {
        "pdf".to_string()
    } else if lower.ends_with(".mp4") || lower.ends_with(".webm") || lower.ends_with(".mkv")
        || lower.ends_with(".avi") || lower.ends_with(".mov") || lower.ends_with(".ogv")
    {
        "video".to_string()
    } else if lower.ends_with(".doc") || lower.ends_with(".docx") {
        "word".to_string()
    } else if lower.ends_with(".xls") || lower.ends_with(".xlsx") {
        "excel".to_string()
    } else if lower.ends_with(".ppt") || lower.ends_with(".pptx") {
        "powerpoint".to_string()
    } else {
        "other".to_string()
    }
}

#[tauri::command]
pub async fn get_files(space_id: String, state: State<'_, AppState>) -> Result<Vec<AppFile>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, space_id, name, original_path, stored_path, file_type, size, created_at FROM files WHERE space_id=?1 ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let files = stmt
        .query_map(rusqlite::params![space_id], |row| {
            Ok(AppFile {
                id: row.get(0)?,
                space_id: row.get(1)?,
                name: row.get(2)?,
                original_path: row.get(3)?,
                stored_path: row.get(4)?,
                file_type: row.get(5)?,
                size: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(files)
}

#[tauri::command]
pub async fn import_file(
    space_id: String,
    source_path: String,
    state: State<'_, AppState>,
) -> Result<AppFile, String> {
    let src = Path::new(&source_path);
    if !src.exists() {
        return Err(format!("File not found: {}", source_path));
    }

    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let files_dir = state.files_dir.join(&space_id);
    fs::create_dir_all(&files_dir).map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stored_name = format!("{}_{}.{}", id, file_name.replace(' ', "_"), ext);
    let dest = files_dir.join(&stored_name);

    fs::copy(src, &dest).map_err(|e| e.to_string())?;

    let size = dest.metadata().map(|m| m.len() as i64).unwrap_or(0);
    let file_type = detect_file_type(&source_path);
    let now = Utc::now().to_rfc3339();
    let stored_path = dest.to_string_lossy().to_string();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO files (id, space_id, name, original_path, stored_path, file_type, size, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        rusqlite::params![id, space_id, file_name, source_path, stored_path, file_type, size, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(AppFile {
        id,
        space_id,
        name: file_name,
        original_path: source_path,
        stored_path,
        file_type,
        size,
        created_at: now,
    })
}

#[tauri::command]
pub async fn delete_file(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let stored_path: String = db
        .query_row("SELECT stored_path FROM files WHERE id=?1", rusqlite::params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    // Delete physical file (ignore error if already gone)
    let _ = fs::remove_file(&stored_path);

    db.execute("DELETE FROM files WHERE id=?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Save raw bytes (from clipboard) as an image file in the space's files directory
#[tauri::command]
pub async fn save_image_bytes(
    space_id: String,
    bytes: Vec<u8>,
    ext: String,
    state: State<'_, AppState>,
) -> Result<AppFile, String> {
    let files_dir = state.files_dir.join(&space_id);
    fs::create_dir_all(&files_dir).map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    // ext may come as "image/png" → strip to just "png"
    let clean_ext = ext.split('/').last().unwrap_or("png");
    let file_name = format!("clipboard_{}.{}", &id[..8], clean_ext);
    let dest = files_dir.join(&file_name);

    fs::write(&dest, &bytes).map_err(|e| e.to_string())?;

    let size = bytes.len() as i64;
    let now = Utc::now().to_rfc3339();
    let stored_path = dest.to_string_lossy().to_string();

    Ok(AppFile {
        id,
        space_id,
        name: file_name,
        original_path: "clipboard".to_string(),
        stored_path,
        file_type: "image".to_string(),
        size,
        created_at: now,
    })
}

#[tauri::command]
pub async fn read_file_as_base64(path: String) -> Result<String, String> {
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    Ok(base64_encode(&data))
}

/// Open a local file with the OS default application
#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── OG metadata fetcher ─────────────────────────────────

#[derive(serde::Serialize)]
pub struct OgMeta {
    pub title: String,
    pub description: String,
    pub image: String,
    pub site_name: String,
}

#[tauri::command]
pub async fn fetch_og_meta(url: String) -> Result<OgMeta, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; OmniSpace/1.0; +https://omnispace.app)")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Error de red: {}", e))?;

    let html = resp.text().await.map_err(|e| e.to_string())?;

    fn extract_meta(html: &str, property: &str) -> String {
        // Handle both attribute orders: property/name first, then content; or content first
        let props = [
            format!(r#"property="{}" content=""#, property),
            format!(r#"name="{}" content=""#, property),
            format!(r#"property='{}' content='"#, property),
        ];
        for pat in &props {
            if let Some(idx) = html.to_lowercase().find(&pat.to_lowercase()) {
                let rest = &html[idx + pat.len()..];
                let end = rest.find(|c| c == '"' || c == '\'').unwrap_or(0);
                if end > 0 { return rest[..end].trim().to_string(); }
            }
        }
        // Reversed: content first, then property
        let rev = format!(r#"content=""#);
        let prop_check = format!(r#"property="{}""#, property);
        let mut search = &html[..];
        while let Some(ci) = search.find(&rev) {
            let content_start = ci + rev.len();
            let before = &search[..ci];
            let meta_start = before.rfind("<meta").unwrap_or(0);
            let meta_chunk = &search[meta_start..content_start + 200.min(search.len() - content_start)];
            if meta_chunk.to_lowercase().contains(&prop_check.to_lowercase()) {
                let rest = &search[content_start..];
                let end = rest.find('"').unwrap_or(0);
                if end > 0 { return rest[..end].trim().to_string(); }
            }
            if ci + 1 >= search.len() { break; }
            search = &search[ci + 1..];
        }
        String::new()
    }

    let og_title = extract_meta(&html, "og:title");
    let title = if og_title.is_empty() {
        // Fallback to <title>
        html.find("<title>")
            .and_then(|s| {
                let rest = &html[s + 7..];
                rest.find("</title>").map(|e| rest[..e].trim().to_string())
            })
            .unwrap_or_default()
    } else {
        og_title
    };

    Ok(OgMeta {
        title:       decode_entities(&title),
        description: decode_entities(&extract_meta(&html, "og:description")),
        image:       extract_meta(&html, "og:image"),
        site_name:   decode_entities(&extract_meta(&html, "og:site_name")),
    })
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp + 1..];
        if let Some(semi) = rest.find(';') {
            let entity = &rest[..semi];
            let decoded = match entity {
                "amp"  => Some('&'),
                "lt"   => Some('<'),
                "gt"   => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{00A0}'),
                e if e.starts_with("#x") || e.starts_with("#X") => {
                    u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32)
                }
                e if e.starts_with('#') => {
                    e[1..].parse::<u32>().ok().and_then(char::from_u32)
                }
                _ => None,
            };
            if let Some(ch) = decoded {
                out.push(ch);
                rest = &rest[semi + 1..];
            } else {
                out.push('&');
            }
        } else {
            out.push('&');
        }
    }
    out.push_str(rest);
    out
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        result.push(CHARS[((b0 >> 2) & 0x3F) as usize] as char);
        result.push(CHARS[(((b0 << 4) | (b1 >> 4)) & 0x3F) as usize] as char);
        result.push(if i + 1 < data.len() { CHARS[(((b1 << 2) | (b2 >> 6)) & 0x3F) as usize] as char } else { '=' });
        result.push(if i + 2 < data.len() { CHARS[(b2 & 0x3F) as usize] as char } else { '=' });
        i += 3;
    }
    result
}
