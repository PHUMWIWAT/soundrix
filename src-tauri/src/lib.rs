mod commands;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let library = commands::open_library(app.handle())
                .map_err(|error| std::io::Error::other(error))?;
            app.manage(library);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library_get,
            commands::library_warning,
            commands::import_paths,
            commands::add_category,
            commands::remove_category,
            commands::add_playlist,
            commands::remove_playlist,
            commands::move_sound,
            commands::remove_sound,
            commands::add_to_playlist,
            commands::remove_from_playlist,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
