// Application entry point for the Tauri core.
//
// Commands are added here as features land (Step 4+). Keeping the handler
// explicit and empty for now - every exposed command is an attack surface,
// so we add them deliberately, one at a time, with validated inputs.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
