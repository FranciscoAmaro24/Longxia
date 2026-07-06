// Application entry point for the Tauri host.
//
// All real logic lives in the `longxia_core` crate; this binary only owns the
// database connection as managed state and exposes the core operations as
// typed Tauri commands. Every exposed command is an attack surface, so we
// register them deliberately.

pub mod commands;

use std::sync::Mutex;
use tauri::Manager;

use rusqlite::Connection;

/// Managed state wrapper around the single app connection. Held behind a Mutex
/// because Tauri commands may run concurrently; the core operations take a
/// plain `&Connection` and stay unaware of how the host holds it.
pub struct Db(pub Mutex<Connection>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Store the SQLite file in the OS app-data directory.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = longxia_core::db::init(&dir.join("longxia.db"))?;
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_summary,
            commands::lookup,
            commands::annotate,
            commands::get_review_queue,
            commands::review_card,
            commands::explain,
            commands::get_note,
            commands::save_note,
            commands::add_insight,
            commands::delete_insight
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
