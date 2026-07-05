// Application entry point for the Tauri core.
//
// The database lives here as managed state; features reach it only through the
// typed commands in `commands`. Every exposed command is an attack surface, so
// we register them deliberately.

pub mod commands;
pub mod db;
pub mod dict_import;
pub mod error;
pub mod models;
pub mod srs;

use std::sync::Mutex;
use tauri::Manager;

use db::Db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Store the SQLite file in the OS app-data directory.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::init(&dir.join("longxia.db"))?;
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_summary,
            commands::lookup,
            commands::annotate,
            commands::get_review_queue,
            commands::review_card
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
