mod audio;
mod commands;
mod decode;
mod shortcuts;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(shortcuts::on_shortcut)
                .build(),
        )
        .setup(|app| {
            let library = commands::open_library(app.handle())
                .map_err(|error| std::io::Error::other(error))?;
            let engine = audio::Engine::open(app.handle())
                .map_err(|error| std::io::Error::other(error))?;
            app.manage(library);
            app.manage(audio::AudioState(Mutex::new(engine)));
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let _ = shortcuts::sync(&handle);
            });
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
            commands::set_shortcut,
            commands::add_to_playlist,
            commands::remove_from_playlist,
            commands::audio_status,
            commands::set_audio,
            commands::play_sound,
            commands::stop_sounds,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
