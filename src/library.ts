import { invoke } from "@tauri-apps/api/core";

export type Sound = {
  id: string;
  name: string;
  path: string;
  categoryId: string;
  shortcut: string;
};

export type Category = {
  id: string;
  name: string;
};

export type Playlist = {
  id: string;
  name: string;
  soundIds: string[];
};

export type Library = {
  categories: Category[];
  sounds: Sound[];
  playlists: Playlist[];
};

export const AUDIO_EXTENSIONS = ["aac", "flac", "m4a", "mp3", "ogg", "opus", "wav"];

export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function loadLibrary(): Promise<Library> {
  return invoke<Library>("library_get");
}

export function loadWarning(): Promise<string | null> {
  return invoke<string | null>("library_warning");
}

export function importPaths(paths: string[], categoryId: string): Promise<Library> {
  return invoke<Library>("import_paths", { paths, categoryId });
}

export function addCategory(name: string): Promise<Library> {
  return invoke<Library>("add_category", { name });
}

export function removeCategory(categoryId: string): Promise<Library> {
  return invoke<Library>("remove_category", { categoryId });
}

export function addPlaylist(name: string): Promise<Library> {
  return invoke<Library>("add_playlist", { name });
}

export function removePlaylist(playlistId: string): Promise<Library> {
  return invoke<Library>("remove_playlist", { playlistId });
}

export function moveSound(soundId: string, categoryId: string): Promise<Library> {
  return invoke<Library>("move_sound", { soundId, categoryId });
}

export function removeSound(soundId: string): Promise<Library> {
  return invoke<Library>("remove_sound", { soundId });
}

export function setShortcut(soundId: string, shortcut: string): Promise<Library> {
  return invoke<Library>("set_shortcut", { soundId, shortcut });
}

export function addToPlaylist(playlistId: string, soundId: string): Promise<Library> {
  return invoke<Library>("add_to_playlist", { playlistId, soundId });
}

export function removeFromPlaylist(playlistId: string, soundId: string): Promise<Library> {
  return invoke<Library>("remove_from_playlist", { playlistId, soundId });
}
