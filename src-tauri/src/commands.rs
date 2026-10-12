use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};

use soundrix_core::{AppLibrary, Library};

pub(crate) struct LibraryState(Mutex<AppLibrary>);

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
pub fn remove_sound(state: State<'_, LibraryState>, sound_id: String) -> Result<Library, String> {
    with_library(&state, |library| {
        library
            .library_mut()
            .remove_sound(&sound_id)
            .map_err(|error| error.to_string())?;
        Ok(snapshot(library))
    })
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
