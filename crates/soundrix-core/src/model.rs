use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const AUDIO_EXTENSIONS: &[&str] = &["aac", "flac", "m4a", "mp3", "ogg", "opus", "wav"];

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("unknown category")]
    UnknownCategory,
    #[error("unknown sound")]
    UnknownSound,
    #[error("unknown playlist")]
    UnknownPlaylist,
    #[error("name is empty")]
    EmptyName,
    #[error("keep at least one category")]
    LastCategory,
    #[error("that shortcut is already used")]
    ShortcutTaken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub path: String,
    pub category_id: String,
    #[serde(default)]
    pub shortcut: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub sound_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub categories: Vec<Category>,
    pub sounds: Vec<Sound>,
    pub playlists: Vec<Playlist>,
    next_id: u64,
}

impl Library {
    pub fn new() -> Self {
        let mut library = Self {
            categories: Vec::new(),
            sounds: Vec::new(),
            playlists: Vec::new(),
            next_id: 1,
        };
        library
            .add_category("Library")
            .expect("the first category always fits");
        library
    }

    pub fn add_category(&mut self, name: &str) -> Result<String, LibraryError> {
        let name = trimmed(name)?;
        let id = self.allocate_id();
        self.categories.push(Category {
            id: id.clone(),
            name,
        });
        Ok(id)
    }

    pub fn remove_category(&mut self, category_id: &str) -> Result<(), LibraryError> {
        if self.categories.len() <= 1 {
            return Err(LibraryError::LastCategory);
        }
        let Some(index) = self.categories.iter().position(|category| category.id == category_id)
        else {
            return Err(LibraryError::UnknownCategory);
        };
        self.categories.remove(index);
        let fallback = self.categories[0].id.clone();
        for sound in &mut self.sounds {
            if sound.category_id == category_id {
                sound.category_id = fallback.clone();
            }
        }
        Ok(())
    }

    pub fn add_playlist(&mut self, name: &str) -> Result<String, LibraryError> {
        let name = trimmed(name)?;
        let id = self.allocate_id();
        self.playlists.push(Playlist {
            id: id.clone(),
            name,
            sound_ids: Vec::new(),
        });
        Ok(id)
    }

    pub fn remove_playlist(&mut self, playlist_id: &str) -> Result<(), LibraryError> {
        let Some(index) = self
            .playlists
            .iter()
            .position(|playlist| playlist.id == playlist_id)
        else {
            return Err(LibraryError::UnknownPlaylist);
        };
        self.playlists.remove(index);
        Ok(())
    }

    pub fn import_paths(
        &mut self,
        paths: &[String],
        category_id: &str,
    ) -> Result<usize, LibraryError> {
        self.category(category_id)?;
        let mut found = Vec::new();
        for path in paths {
            collect(Path::new(path), &mut found, true)?;
        }
        let mut added = 0;
        for path in found {
            let text = path.to_string_lossy().to_string();
            if self.sounds.iter().any(|sound| sound.path == text) {
                continue;
            }
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .filter(|stem| !stem.is_empty())
                .unwrap_or_else(|| text.clone());
            let id = self.allocate_id();
            self.sounds.push(Sound {
                id,
                name,
                path: text,
                category_id: category_id.to_string(),
                shortcut: String::new(),
            });
            added += 1;
        }
        Ok(added)
    }

    pub fn set_shortcut(&mut self, sound_id: &str, shortcut: &str) -> Result<(), LibraryError> {
        let shortcut = shortcut.trim().to_string();
        if !shortcut.is_empty()
            && self.sounds.iter().any(|sound| {
                sound.id != sound_id && sound.shortcut.eq_ignore_ascii_case(&shortcut)
            })
        {
            return Err(LibraryError::ShortcutTaken);
        }
        let sound = self
            .sounds
            .iter_mut()
            .find(|sound| sound.id == sound_id)
            .ok_or(LibraryError::UnknownSound)?;
        sound.shortcut = shortcut;
        Ok(())
    }

    pub fn move_sound(&mut self, sound_id: &str, category_id: &str) -> Result<(), LibraryError> {
        self.category(category_id)?;
        let sound = self
            .sounds
            .iter_mut()
            .find(|sound| sound.id == sound_id)
            .ok_or(LibraryError::UnknownSound)?;
        sound.category_id = category_id.to_string();
        Ok(())
    }

    pub fn remove_sound(&mut self, sound_id: &str) -> Result<(), LibraryError> {
        let Some(index) = self.sounds.iter().position(|sound| sound.id == sound_id) else {
            return Err(LibraryError::UnknownSound);
        };
        self.sounds.remove(index);
        for playlist in &mut self.playlists {
            playlist.sound_ids.retain(|id| id != sound_id);
        }
        Ok(())
    }

    pub fn add_to_playlist(
        &mut self,
        playlist_id: &str,
        sound_id: &str,
    ) -> Result<(), LibraryError> {
        if !self.sounds.iter().any(|sound| sound.id == sound_id) {
            return Err(LibraryError::UnknownSound);
        }
        let playlist = self
            .playlists
            .iter_mut()
            .find(|playlist| playlist.id == playlist_id)
            .ok_or(LibraryError::UnknownPlaylist)?;
        if !playlist.sound_ids.iter().any(|id| id == sound_id) {
            playlist.sound_ids.push(sound_id.to_string());
        }
        Ok(())
    }

    pub fn remove_from_playlist(
        &mut self,
        playlist_id: &str,
        sound_id: &str,
    ) -> Result<(), LibraryError> {
        let playlist = self
            .playlists
            .iter_mut()
            .find(|playlist| playlist.id == playlist_id)
            .ok_or(LibraryError::UnknownPlaylist)?;
        playlist.sound_ids.retain(|id| id != sound_id);
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn matching(&self, query: &str, category_id: Option<&str>) -> Vec<&Sound> {
        let query = query.trim().to_lowercase();
        self.sounds
            .iter()
            .filter(|sound| category_id.is_none_or(|id| sound.category_id == id))
            .filter(|sound| query.is_empty() || sound.name.to_lowercase().contains(&query))
            .collect()
    }

    fn category(&self, category_id: &str) -> Result<(), LibraryError> {
        if self.categories.iter().any(|category| category.id == category_id) {
            Ok(())
        } else {
            Err(LibraryError::UnknownCategory)
        }
    }

    fn allocate_id(&mut self) -> String {
        let id = self.next_id.to_string();
        self.next_id += 1;
        id
    }
}

fn trimmed(name: &str) -> Result<String, LibraryError> {
    let name = name.trim();
    if name.is_empty() {
        Err(LibraryError::EmptyName)
    } else {
        Ok(name.to_string())
    }
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            AUDIO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
}

fn collect(path: &Path, found: &mut Vec<PathBuf>, top: bool) -> Result<(), LibraryError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if top => {
            return Err(LibraryError::Read {
                path: path.to_string_lossy().to_string(),
                source,
            });
        }
        Err(_) => return Ok(()),
    };
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_dir() {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(source) if top => {
                return Err(LibraryError::Read {
                    path: path.to_string_lossy().to_string(),
                    source,
                });
            }
            Err(_) => return Ok(()),
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            collect(&entry.path(), found, false)?;
        }
        return Ok(());
    }
    if is_audio(path) {
        found.push(path.to_path_buf());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "soundrix-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn import_should_add_audio_and_skip_duplicates_and_other_files() {
        let dir = scratch("import");
        let wav = dir.join("Horn.wav");
        let mp3 = dir.join("nested");
        std::fs::create_dir_all(&mp3).expect("nested");
        std::fs::write(&wav, b"wav").expect("wav");
        std::fs::write(mp3.join("Hit.MP3"), b"mp3").expect("mp3");
        std::fs::write(dir.join("notes.txt"), b"no").expect("txt");

        let mut library = Library::new();
        let category = library.categories[0].id.clone();
        let added = library
            .import_paths(&[dir.to_string_lossy().to_string()], &category)
            .expect("import");
        assert_eq!(added, 2);
        let again = library
            .import_paths(&[wav.to_string_lossy().to_string()], &category)
            .expect("second import");
        assert_eq!(again, 0);
        assert_eq!(library.sounds.len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn import_should_fail_when_the_path_is_missing() {
        let mut library = Library::new();
        let category = library.categories[0].id.clone();
        let error = library
            .import_paths(&["C:/soundrix-missing-file.wav".to_string()], &category)
            .expect_err("missing");
        assert!(matches!(error, LibraryError::Read { .. }));
    }

    #[test]
    fn search_should_match_name_inside_the_selected_category() {
        let mut library = Library::new();
        let library_id = library.categories[0].id.clone();
        let memes = library.add_category("Memes").expect("category");
        library.sounds.push(Sound {
            id: "s1".into(),
            name: "Air Horn".into(),
            path: "a.wav".into(),
            category_id: library_id,
            shortcut: String::new(),
        });
        library.sounds.push(Sound {
            id: "s2".into(),
            name: "horn solo".into(),
            path: "b.wav".into(),
            category_id: memes.clone(),
            shortcut: String::new(),
        });
        let hits = library.matching("HORN", Some(&memes));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "s2");
    }

    #[test]
    fn remove_category_should_keep_sounds_in_the_remaining_category() {
        let mut library = Library::new();
        let original = library.categories[0].id.clone();
        let extra = library.add_category("Extra").expect("category");
        library.sounds.push(Sound {
            id: "s1".into(),
            name: "Hit".into(),
            path: "hit.wav".into(),
            category_id: extra.clone(),
            shortcut: String::new(),
        });
        library.remove_category(&extra).expect("remove");
        assert_eq!(library.sounds[0].category_id, original);
        assert!(library.remove_category(&original).is_err());
    }

    #[test]
    fn playlist_should_drop_a_sound_when_that_sound_is_removed() {
        let mut library = Library::new();
        library.sounds.push(Sound {
            id: "s1".into(),
            name: "Hit".into(),
            path: "hit.wav".into(),
            category_id: library.categories[0].id.clone(),
            shortcut: String::new(),
        });
        let playlist = library.add_playlist("Warmup").expect("playlist");
        library.add_to_playlist(&playlist, "s1").expect("add");
        library.add_to_playlist(&playlist, "s1").expect("again");
        assert_eq!(library.playlists[0].sound_ids.len(), 1);
        library.remove_sound("s1").expect("remove sound");
        assert!(library.playlists[0].sound_ids.is_empty());
    }

    #[test]
    fn set_shortcut_should_reject_a_key_already_used_by_another_sound() {
        let mut library = Library::new();
        library.sounds.push(Sound {
            id: "s1".into(),
            name: "Hit".into(),
            path: "a.wav".into(),
            category_id: library.categories[0].id.clone(),
            shortcut: String::new(),
        });
        library.sounds.push(Sound {
            id: "s2".into(),
            name: "Horn".into(),
            path: "b.wav".into(),
            category_id: library.categories[0].id.clone(),
            shortcut: String::new(),
        });
        library.set_shortcut("s1", "Ctrl+Shift+K").expect("first key");
        let taken = library.set_shortcut("s2", "ctrl+shift+k");
        assert!(matches!(taken, Err(LibraryError::ShortcutTaken)));
        library.set_shortcut("s1", "").expect("clear");
        assert!(library.sounds[0].shortcut.is_empty());
    }

    #[test]
    fn empty_names_should_be_rejected() {
        let mut library = Library::new();
        assert!(library.add_category("  ").is_err());
        assert!(library.add_playlist("").is_err());
    }
}
