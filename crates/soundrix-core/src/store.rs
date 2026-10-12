use std::path::{Path, PathBuf};

use crate::model::Library;

pub struct AppLibrary {
    path: PathBuf,
    library: Library,
    pub warning: Option<String>,
}

impl AppLibrary {
    pub fn load(path: PathBuf) -> Self {
        if !path.exists() {
            return Self {
                path,
                library: Library::new(),
                warning: None,
            };
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Library>(&text) {
                Ok(library) => Self {
                    path,
                    library,
                    warning: None,
                },
                Err(error) => Self::replaced(path, error.to_string()),
            },
            Err(error) => Self {
                path,
                library: Library::new(),
                warning: Some(format!("could not read the library: {error}")),
            },
        }
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn library_mut(&mut self) -> &mut Library {
        &mut self.library
    }

    pub fn save(&mut self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let text = serde_json::to_string_pretty(&self.library).map_err(|error| error.to_string())?;
        std::fs::write(&self.path, text).map_err(|error| error.to_string())?;
        self.warning = None;
        Ok(())
    }

    fn replaced(path: PathBuf, reason: String) -> Self {
        let backup = backup_path(&path);
        let warning = match std::fs::rename(&path, &backup) {
            Ok(()) => format!(
                "library file was unreadable ({reason}). Kept a copy at {}.",
                backup.display()
            ),
            Err(error) => format!("library file was unreadable ({reason}), and the copy failed: {error}"),
        };
        Self {
            path,
            library: Library::new(),
            warning: Some(warning),
        }
    }
}

fn backup_path(path: &Path) -> PathBuf {
    let mut backup = path.to_path_buf();
    backup.set_extension("json.bak");
    backup
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_should_round_trip_the_library() {
        let dir = std::env::temp_dir().join(format!(
            "soundrix-store-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("library.json");
        let mut store = AppLibrary::load(path.clone());
        store
            .library_mut()
            .add_playlist("Warmup")
            .expect("playlist");
        store.save().expect("save");
        let loaded = AppLibrary::load(path);
        assert_eq!(loaded.library().playlists[0].name, "Warmup");
        assert!(loaded.warning.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_should_keep_a_backup_when_the_file_is_not_json() {
        let dir = std::env::temp_dir().join(format!("soundrix-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("library.json");
        std::fs::write(&path, b"not-json").expect("write");
        let store = AppLibrary::load(path.clone());
        assert!(store.warning.is_some());
        assert!(dir.join("library.json.bak").exists());
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
