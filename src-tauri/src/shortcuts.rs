use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

use crate::commands::{self, LibraryState};

static SHORTCUT_LOCK: Mutex<()> = Mutex::new(());

pub fn sync(app: &AppHandle) -> Result<(), String> {
    let _guard = SHORTCUT_LOCK.lock().map_err(|error| error.to_string())?;
    let shortcuts = {
        let library = app.state::<LibraryState>();
        let library = library.0.lock().map_err(|error| error.to_string())?;
        library
            .library()
            .sounds
            .iter()
            .filter(|sound| !sound.shortcut.is_empty())
            .map(|sound| sound.shortcut.clone())
            .collect::<Vec<_>>()
    };
    let global = app.global_shortcut();
    global.unregister_all().map_err(|error| error.to_string())?;
    for shortcut in shortcuts {
        global
            .register(shortcut.as_str())
            .map_err(|error| format!("{shortcut} is already used by another app ({error})"))?;
    }
    Ok(())
}

pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, event: tauri_plugin_global_shortcut::ShortcutEvent) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    if app.try_state::<LibraryState>().is_none() {
        return;
    }
    let pressed = shortcut.to_string();
    let id = {
        let library = app.state::<LibraryState>();
        let Ok(library) = library.0.lock() else {
            return;
        };
        library
            .library()
            .sounds
            .iter()
            .find(|sound| sound.shortcut == pressed)
            .map(|sound| sound.id.clone())
    };
    let Some(id) = id else {
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = commands::play_saved(&app, id, None).await;
    });
}

pub fn canonical(shortcut: &str) -> Result<String, String> {
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        return Ok(String::new());
    }
    let parsed: Shortcut = shortcut
        .parse()
        .map_err(|_| "That key cannot be used as a shortcut.".to_string())?;
    if parsed.mods.is_empty() && !is_function_key(parsed.key) {
        return Err(
            "Use Ctrl, Alt, or Shift with that key, or use a function key on its own.".to_string(),
        );
    }
    Ok(parsed.to_string())
}

fn is_function_key(key: Code) -> bool {
    matches!(
        key,
        Code::F1
            | Code::F2
            | Code::F3
            | Code::F4
            | Code::F5
            | Code::F6
            | Code::F7
            | Code::F8
            | Code::F9
            | Code::F10
            | Code::F11
            | Code::F12
            | Code::F13
            | Code::F14
            | Code::F15
            | Code::F16
            | Code::F17
            | Code::F18
            | Code::F19
            | Code::F20
            | Code::F21
            | Code::F22
            | Code::F23
            | Code::F24
    )
}
