use std::path::Path;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};

use soundrix_core::{AppLibrary, Library};

use crate::audio::{self, AudioState, AudioStatus};
use crate::decode;

pub(crate) struct LibraryState(pub(crate) Mutex<AppLibrary>);

pub(crate) fn open_library(app: &AppHandle) -> Result<LibraryState, String> {
    let dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
    Ok(LibraryState(Mutex::new(AppLibrary::load(dir.join("library.json")))))
}

fn with_library<T>(
    state: &State<'_, LibraryState>,
    change: impl FnOnce(&mut AppLibrary) -> Result<T, String>,
) -> Result<T, String> {
    let mut library = state.0.lock().map_err(|error| error.to_string())?;
    let value = change(&mut library)?;
    library.save()?;
    Ok(value)
}

fn snapshot(library: &AppLibrary) -> Library {
    library.library().clone()
}

#[tauri::command]
pub fn library_get(state: State<'_, LibraryState>) -> Result<Library, String> {
    let library = state.0.lock().map_err(|error| error.to_string())?;
    Ok(snapshot(&library))
}

#[tauri::command]
pub fn library_warning(state: State<'_, LibraryState>) -> Result<Option<String>, String> {
    let library = state.0.lock().map_err(|error| error.to_string())?;
    Ok(library.warning.clone())
}

#[tauri::command]
pub fn import_paths(
    state: State<'_, LibraryState>,
    paths: Vec<String>,
    category_id: String,
) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .import_paths(&paths, &category_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn add_category(state: State<'_, LibraryState>, name: String) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .add_category(&name)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn remove_category(
    state: State<'_, LibraryState>,
    category_id: String,
) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .remove_category(&category_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn add_playlist(state: State<'_, LibraryState>, name: String) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .add_playlist(&name)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn remove_playlist(
    state: State<'_, LibraryState>,
    playlist_id: String,
) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .remove_playlist(&playlist_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn move_sound(
    state: State<'_, LibraryState>,
    sound_id: String,
    category_id: String,
) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .move_sound(&sound_id, &category_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub async fn remove_sound(
    app: AppHandle,
    state: State<'_, LibraryState>,
    sound_id: String,
) -> Result<Library, String> {
    let library = with_library(&state, |library| {
        library
            .library_mut()
            .remove_sound(&sound_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })?;
    crate::shortcuts::sync(&app)?;
    Ok(library)
}

#[tauri::command]
pub async fn set_shortcut(
    app: AppHandle,
    state: State<'_, LibraryState>,
    sound_id: String,
    shortcut: String,
) -> Result<Library, String> {
    let shortcut = crate::shortcuts::canonical(&shortcut)?;
    let previous = {
        let library = state.0.lock().map_err(|error| error.to_string())?;
        library
            .library()
            .sounds
            .iter()
            .find(|sound| sound.id == sound_id)
            .map(|sound| sound.shortcut.clone())
            .ok_or_else(|| "That sound is no longer in the library.".to_string())?
    };
    let saved = with_library(&state, |library| {
        library
            .library_mut()
            .set_shortcut(&sound_id, &shortcut)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })?;
    if let Err(error) = crate::shortcuts::sync(&app) {
        let _ = with_library(&state, |library| {
            library
                .library_mut()
                .set_shortcut(&sound_id, &previous)
                .map_err(|error| error.to_string())?;
            Ok(())
        });
        let _ = crate::shortcuts::sync(&app);
        return Err(error);
    }
    Ok(saved)
}

#[tauri::command]
pub fn add_to_playlist(
    state: State<'_, LibraryState>,
    playlist_id: String,
    sound_id: String,
) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .add_to_playlist(&playlist_id, &sound_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn remove_from_playlist(
    state: State<'_, LibraryState>,
    playlist_id: String,
    sound_id: String,
) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .remove_from_playlist(&playlist_id, &sound_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
}

#[tauri::command]
pub fn audio_status(audio: State<'_, AudioState>) -> Result<AudioStatus, String> {
    let mut engine = audio.0.lock().map_err(|error| error.to_string())?;
    Ok(engine.status())
}

#[tauri::command]
pub fn set_audio(
    audio: State<'_, AudioState>,
    mode: String,
    speaker_device: String,
    mic_device: String,
) -> Result<AudioStatus, String> {
    let mut engine = audio.0.lock().map_err(|error| error.to_string())?;
    engine.apply(audio::parse_mode(&mode)?, speaker_device, mic_device)
}

pub async fn play_saved(
    app: &AppHandle,
    id: String,
    mode: Option<String>,
) -> Result<AudioStatus, String> {
    let path = {
        let library = app.state::<LibraryState>();
        let library = library.0.lock().map_err(|error| error.to_string())?;
        library
            .library()
            .sounds
            .iter()
            .find(|sound| sound.id == id)
            .map(|sound| sound.path.clone())
            .ok_or_else(|| "That sound is no longer in the library.".to_string())?
    };
    let pcm = tauri::async_runtime::spawn_blocking(move || decode::decode_file(Path::new(&path)))
        .await
        .map_err(|error| error.to_string())??;
    let audio = app.state::<AudioState>();
    let mut engine = audio.0.lock().map_err(|error| error.to_string())?;
    let route = match mode {
        Some(mode) => audio::parse_mode(&mode)?,
        None => engine.status().mode,
    };
    engine.play(pcm, id, route)?;
    Ok(engine.status())
}

#[tauri::command]
pub async fn play_sound(
    app: AppHandle,
    id: String,
    mode: Option<String>,
) -> Result<AudioStatus, String> {
    play_saved(&app, id, mode).await
}

#[tauri::command]
pub fn stop_sounds(audio: State<'_, AudioState>) -> Result<AudioStatus, String> {
    let mut engine = audio.0.lock().map_err(|error| error.to_string())?;
    engine.stop();
    Ok(engine.status())
}
