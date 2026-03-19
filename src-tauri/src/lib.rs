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

            let db_path = app_dir.join("omnispace.db");
            let conn = db::init_db(&db_path).expect("Failed to initialize database");

            let files_dir = app_dir.join("files");
            std::fs::create_dir_all(&files_dir).expect("Failed to create files dir");

            app.manage(AppState {
                db: Mutex::new(conn),
                files_dir,
            });

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
            commands::files::import_file,
            commands::files::save_image_bytes,
            commands::files::delete_file,
            commands::files::open_file,
            commands::files::read_file_as_base64,
            commands::files::fetch_og_meta,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
