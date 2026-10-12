import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import logo from "../assets/soundrix-logo.jpg";
import {
  AUDIO_EXTENSIONS,
  addCategory,
  addPlaylist,
  addToPlaylist,
  importPaths,
  inTauri,
  loadLibrary,
  loadWarning,
  moveSound,
  removeCategory,
  removeFromPlaylist,
  removePlaylist,
  removeSound,
  type Library,
} from "./library";
import "./theme.css";

function App() {
  const [library, setLibrary] = useState<Library | null>(null);
  const [warning, setWarning] = useState("");
  const [error, setError] = useState("");
  const [query, setQuery] = useState("");
  const [categoryId, setCategoryId] = useState("all");
  const [playlistId, setPlaylistId] = useState<string | null>(null);
  const [categoryName, setCategoryName] = useState("");
  const [playlistName, setPlaylistName] = useState("");
  const [hot, setHot] = useState(false);
  const libraryRef = useRef(library);
  const categoryRef = useRef(categoryId);
  libraryRef.current = library;
  categoryRef.current = categoryId;

  useEffect(() => {
    if (!inTauri()) {
      return;
    }
    let unlisten: (() => void) | undefined;
    let alive = true;
    void (async () => {
      try {
        const [next, note] = await Promise.all([loadLibrary(), loadWarning()]);
        if (!alive) {
          return;
        }
        setLibrary(next);
        setWarning(note ?? "");
        const webview = getCurrentWebview();
        unlisten = await webview.onDragDropEvent((event) => {
          if (event.payload.type === "enter") {
            setHot(true);
          } else if (event.payload.type === "leave") {
            setHot(false);
          } else if (event.payload.type === "drop") {
            setHot(false);
            void acceptPaths(event.payload.paths);
          }
        });
      } catch (caught) {
        if (alive) {
          setError(message(caught));
        }
      }
    })();
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  async function acceptPaths(paths: string[]) {
    const current = libraryRef.current;
    if (!current || paths.length === 0) {
      return;
    }
    const selected = categoryRef.current;
    const target = selected === "all" ? current.categories[0]?.id : selected;
    if (!target) {
      setError("Add a category before importing sounds.");
      return;
    }
    await run(() => importPaths(paths, target));
  }

  async function run(action: () => Promise<Library>) {
    setError("");
    try {
      setLibrary(await action());
    } catch (caught) {
      setError(message(caught));
    }
  }

  async function pickFiles() {
    if (!inTauri()) {
      setError("Open Soundrix with npm run tauri dev to use the library.");
      return;
    }
    const picked = await open({
      multiple: true,
      filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
    });
    if (picked) {
      await acceptPaths(Array.isArray(picked) ? picked : [picked]);
    }
  }

  async function pickFolder() {
    if (!inTauri()) {
      setError("Open Soundrix with npm run tauri dev to use the library.");
      return;
    }
    const picked = await open({ directory: true, multiple: true });
    if (picked) {
      await acceptPaths(Array.isArray(picked) ? picked : [picked]);
    }
  }

  const playlist = library?.playlists.find((item) => item.id === playlistId) ?? null;
  const needle = query.trim().toLowerCase();
  const sounds = (library?.sounds ?? []).filter((sound) => {
    const inCategory = categoryId === "all" || sound.categoryId === categoryId;
    const inPlaylist = playlist === null || playlist.soundIds.includes(sound.id);
    const named = needle === "" || sound.name.toLowerCase().includes(needle);
    return inCategory && inPlaylist && named;
  });

  return (
    <main className="shell">
      <header className="top">
        <img className="mark" src={logo} alt="" />
        <div>
          <h1>Soundrix</h1>
          <p>Soundboard</p>
        </div>
      </header>
      <div className="workspace">
        <aside className="side">
          <section>
            <h2>Categories</h2>
            <button
              type="button"
              className={categoryId === "all" ? "selected" : ""}
              onClick={() => setCategoryId("all")}
            >
              All
            </button>
            {library?.categories.map((category) => (
              <div className="line" key={category.id}>
                <button
                  type="button"
                  className={categoryId === category.id ? "selected" : ""}
                  onClick={() => setCategoryId(category.id)}
                >
                  {category.name}
                </button>
                {library.categories.length > 1 && (
                  <button
                    type="button"
                    onClick={() => void run(() => removeCategory(category.id))}
                  >
                    Remove
                  </button>
                )}
              </div>
            ))}
            <form
              className="line"
              onSubmit={(event) => {
                event.preventDefault();
                const name = categoryName;
                setCategoryName("");
                void run(() => addCategory(name));
              }}
            >
              <input
                value={categoryName}
                aria-label="New category"
                onChange={(event) => setCategoryName(event.currentTarget.value)}
              />
              <button type="submit">Add</button>
            </form>
          </section>
          <section>
            <h2>Playlists</h2>
            {library?.playlists.map((item) => (
              <div className="line" key={item.id}>
                <button
                  type="button"
                  className={playlistId === item.id ? "selected" : ""}
                  onClick={() =>
                    setPlaylistId((current) => (current === item.id ? null : item.id))
                  }
                >
                  {item.name}
                </button>
                <button type="button" onClick={() => void run(() => removePlaylist(item.id))}>
                  Remove
                </button>
              </div>
            ))}
            <form
              className="line"
              onSubmit={(event) => {
                event.preventDefault();
                const name = playlistName;
                setPlaylistName("");
                void run(() => addPlaylist(name));
              }}
            >
              <input
                value={playlistName}
                aria-label="New playlist"
                onChange={(event) => setPlaylistName(event.currentTarget.value)}
              />
              <button type="submit">Add</button>
            </form>
          </section>
        </aside>
        <section
          className={hot ? "board hot" : "board"}
          aria-label="Sound list"
        >
          <div className="toolbar">
            <input
              value={query}
              placeholder="Search sounds"
              aria-label="Search sounds"
              onChange={(event) => setQuery(event.currentTarget.value)}
            />
            <button type="button" onClick={() => void pickFiles()}>
              Add sounds
            </button>
            <button type="button" onClick={() => void pickFolder()}>
              Add folder
            </button>
          </div>
          {warning && <p className="note">{warning}</p>}
          {error && <p className="note">{error}</p>}
          {sounds.length === 0 ? (
            <p className="empty">Drop audio files here, or add them from a folder.</p>
          ) : (
            <ul className="sounds">
              {sounds.map((sound) => (
                <li key={sound.id}>
                  <div>
                    <strong>{sound.name}</strong>
                    <span>{sound.path}</span>
                  </div>
                  <div className="line">
                    <select
                      aria-label={`Category for ${sound.name}`}
                      value={sound.categoryId}
                      onChange={(event) =>
                        void run(() => moveSound(sound.id, event.currentTarget.value))
                      }
                    >
                      {library?.categories.map((category) => (
                        <option key={category.id} value={category.id}>
                          {category.name}
                        </option>
                      ))}
                    </select>
                    {playlist && !playlist.soundIds.includes(sound.id) && (
                      <button
                        type="button"
                        onClick={() => void run(() => addToPlaylist(playlist.id, sound.id))}
                      >
                        Add to playlist
                      </button>
                    )}
                    {playlist?.soundIds.includes(sound.id) && (
                      <button
                        type="button"
                        onClick={() =>
                          void run(() => removeFromPlaylist(playlist.id, sound.id))
                        }
                      >
                        Remove from playlist
                      </button>
                    )}
                    <button type="button" onClick={() => void run(() => removeSound(sound.id))}>
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </main>
  );
}

function message(caught: unknown): string {
  if (caught instanceof Error) {
    return caught.message;
  }
  return String(caught);
}

export default App;
