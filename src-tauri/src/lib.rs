mod commands;
mod db;
mod models;

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;


pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub files_dir: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("stream", |_app, request| {
            use std::io::{Read, Seek, SeekFrom};

            let uri_path = request.uri().path().to_string();
            // URL-decode percent-encoded characters
            let decoded = {
                let mut out = String::with_capacity(uri_path.len());
                let bytes = uri_path.as_bytes();
                let mut i = 0;
                while i < bytes.len() {
                    if bytes[i] == b'%' && i + 2 < bytes.len() {
                        if let Ok(s) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                            if let Ok(b) = u8::from_str_radix(s, 16) {
                                out.push(b as char);
                                i += 3;
                                continue;
                            }
                        }
                    }
                    out.push(bytes[i] as char);
                    i += 1;
                }
                out
            };
            // Strip leading slash (Windows path: /C:/Users/...)
            let path_str = decoded.trim_start_matches('/');
            let path = std::path::Path::new(path_str);

            let meta = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => {
                    return tauri::http::Response::builder()
                        .status(404)
                        .body(vec![])
                        .unwrap()
                }
            };
            let file_size = meta.len();

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let content_type = match ext.as_str() {
                "mp4" | "m4v" => "video/mp4",
                "webm" => "video/webm",
                "mkv" => "video/x-matroska",
                "avi" => "video/x-msvideo",
                "mov" => "video/quicktime",
                "mp3" => "audio/mpeg",
                "wav" => "audio/wav",
                "ogg" => "audio/ogg",
                "m4a" => "audio/mp4",
                "flac" => "audio/flac",
                _ => "application/octet-stream",
            };

            // Parse range header (default to first 2MB if none)
            let (start, end) = if let Some(range_val) = request.headers().get("range") {
                let range_str = range_val.to_str().unwrap_or("");
                if let Some(range) = range_str.strip_prefix("bytes=") {
                    let parts: Vec<&str> = range.split('-').collect();
                    let s: u64 = parts[0].parse().unwrap_or(0);
                    let e: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                        parts[1].parse().unwrap_or(file_size.saturating_sub(1))
                    } else {
                        (s + 2_097_152).min(file_size.saturating_sub(1))
                    };
                    (s, e.min(file_size.saturating_sub(1)))
                } else {
                    (0, (2_097_152u64).min(file_size.saturating_sub(1)))
                }
            } else {
                (0, (2_097_152u64).min(file_size.saturating_sub(1)))
            };

            let length = end - start + 1;

            let mut file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => {
                    return tauri::http::Response::builder()
                        .status(500)
                        .body(vec![])
                        .unwrap()
                }
            };
            let _ = file.seek(SeekFrom::Start(start));

            // Read loop — file.read() may return fewer bytes than requested
            let mut buf = vec![0u8; length as usize];
            let mut total = 0usize;
            while total < buf.len() {
                match file.read(&mut buf[total..]) {
                    Ok(0) => break,
                    Ok(n) => total += n,
                    Err(_) => break,
                }
            }
            buf.truncate(total);
            let actual_end = start + total as u64 - 1;

            tauri::http::Response::builder()
                .status(206)
                .header("Content-Type", content_type)
                .header("Content-Range", format!("bytes {}-{}/{}", start, actual_end, file_size))
                .header("Content-Length", total.to_string())
                .header("Accept-Ranges", "bytes")
                .header("Access-Control-Allow-Origin", "*")
                .body(buf)
                .unwrap()
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");

            let db_name = if cfg!(debug_assertions) { "omnispace_dev.db" } else { "omnispace.db" };
            let db_path = app_dir.join(db_name);
            let conn = db::init_db(&db_path).expect("Failed to initialize database");

            let files_dir = app_dir.join("files");
            std::fs::create_dir_all(&files_dir).expect("Failed to create files dir");

            app.manage(AppState {
                db: Mutex::new(conn),
                files_dir,
            });

            // Fix touchpad pinch-to-zoom in WebView2:
            // wry defaults zoom_hotkeys_enabled=false which sets BOTH
            // SetIsZoomControlEnabled(false) AND SetIsPinchZoomEnabled(false).
            // With IsPinchZoomEnabled=false, WebView2 generates NO synthetic
            // wheel events for pinch gestures — JavaScript never sees them.
            // Fix: re-enable IsPinchZoomEnabled so WebView2 generates
            // synthetic Ctrl+wheel events, while keeping IsZoomControlEnabled=false
            // so WebView2 doesn't apply its own page zoom.
            #[cfg(target_os = "windows")]
            {
                let window = app.get_webview_window("main").unwrap();
                window.with_webview(|wv| unsafe {
                    use webview2_com::Microsoft::Web::WebView2::Win32::{
                        ICoreWebView2Settings5,
                        ICoreWebView2Profile4,
                        ICoreWebView2_13,
                        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
                        COREWEBVIEW2_PERMISSION_KIND_CAMERA,
                        COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
                    };
                    use webview2_com::SetPermissionStateCompletedHandler;
                    use windows::core::Interface;
                    use windows_core::HSTRING;
                    let result = (|| -> windows::core::Result<()> {
                        let settings = wv.controller().CoreWebView2()?.Settings()?;
                        // Keep page zoom disabled (WebView2 won't apply zoom on Ctrl+wheel)
                        settings.SetIsZoomControlEnabled(false)?;
                        // Enable pinch so WebView2 generates synthetic Ctrl+wheel events
                        // that our canvas JavaScript handler can intercept
                        let settings5: ICoreWebView2Settings5 = settings.cast()?;
                        settings5.SetIsPinchZoomEnabled(true)?;
                        // Allow microphone permission automatically so getUserMedia works
                        let wv2 = wv.controller().CoreWebView2()?;
                        // Reset cached mic permission so WebView2 shows its dialog
                        let wv2_13 = wv2.cast::<ICoreWebView2_13>()?;
                        let profile = wv2_13.Profile()?;
                        let profile4 = profile.cast::<ICoreWebView2Profile4>()?;
                        let noop = || SetPermissionStateCompletedHandler::create(Box::new(|_| Ok(())));
                        let origins = [
                            HSTRING::from("http://localhost:5173"),
                            HSTRING::from("tauri://localhost"),
                        ];
                        for origin in &origins {
                            let _ = profile4.SetPermissionState(
                                COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
                                origin,
                                COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
                                &noop(),
                            );
                            let _ = profile4.SetPermissionState(
                                COREWEBVIEW2_PERMISSION_KIND_CAMERA,
                                origin,
                                COREWEBVIEW2_PERMISSION_STATE_DEFAULT,
                                &noop(),
                            );
                        }
                        Ok(())
                    })();
                    if let Err(e) = result {
                        eprintln!("WebView2 zoom config failed: {e:?}");
                    }
                }).ok();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Spaces
            commands::spaces::get_spaces,
            commands::spaces::create_space,
            commands::spaces::update_space,
            commands::spaces::delete_space,
            // Pages
            commands::pages::get_pages,
            commands::pages::create_page,
            commands::pages::update_page_title,
            commands::pages::reorder_pages,
            commands::pages::delete_page,
            commands::pages::export_page,
            commands::pages::import_page,
            // Categories
            commands::categories::get_categories,
            commands::categories::create_category,
            commands::categories::rename_category,
            commands::categories::delete_category,
            commands::categories::assign_space_to_category,
            // Blocks
            commands::blocks::get_blocks,
            commands::blocks::create_block,
            commands::blocks::update_block_position,
            commands::blocks::update_block_content,
            commands::blocks::update_block_zindex,
            commands::blocks::delete_block,
            commands::blocks::restore_block,
            // Strokes
            commands::strokes::get_strokes,
            commands::strokes::save_strokes,
            // Files
            commands::files::get_files,
            commands::files::import_file,
            commands::files::save_image_bytes,
            commands::files::save_audio_bytes,
            commands::files::delete_file,
            commands::files::open_mic_settings,
            commands::files::reset_mic_permission,
            commands::files::save_video_bytes,
            commands::files::reset_camera_permission,
            commands::files::open_file,
            commands::files::read_file_as_base64,
            commands::files::fetch_og_meta,
            // Notes
            commands::notes::get_notes,
            commands::notes::create_note,
            commands::notes::update_note,
            commands::notes::delete_note,
            // Tasks
            commands::tasks::get_tasks,
            commands::tasks::create_task,
            commands::tasks::toggle_task,
            commands::tasks::update_task_title,
            commands::tasks::delete_task,
            // Links
            commands::links::get_links,
            commands::links::create_link,
            commands::links::delete_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
