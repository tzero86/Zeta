# Wave 5B — Bookmarks + Trash

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bookmarks let you save and instantly jump to frequently visited directories, persisted in the config file. Trash replaces permanent deletion with an OS recycle-bin send, making F8 recoverable.

**Jira:** ZTA-131 (ZTA-147 through ZTA-150)

**Wave dependency:** Starts AFTER Wave 5A.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `Cargo.toml` | Add `trash = "3"` |
| Modify | `src/config.rs` | `bookmarks: Vec<PathBuf>` in `AppConfig`; serialise/deserialise |
| Modify | `src/action.rs` | `AddBookmark`, `OpenBookmarks`, `BookmarkSelect(usize)`, `DeleteBookmark(usize)` |
| Modify | `src/state/types.rs` | `ModalKind::Bookmarks` |
| Modify | `src/state/mod.rs` | Bookmark action handlers; save config on bookmark change |
| Modify | `src/fs.rs` | `trash_path(path) -> Result<(), FileSystemError>` |
| Modify | `src/jobs.rs` | `FileOperation::Trash` variant; handle in file-op worker |
| Modify | `src/state/mod.rs` | `OpenDeletePrompt` → send to trash; `FileOperation::Trash` |
| Create | `src/ui/bookmarks.rs` | Render bookmarks modal |
| Modify | `src/ui/mod.rs` | `pub mod bookmarks;`; render in overlay block |

---

## Part A: Bookmarks

### Config

```toml
# In zeta-config.toml
bookmarks = [
    "C:/Users/Zero/Documents/Coding",
    "C:/Users/Zero/Downloads",
]
```

```rust
// In AppConfig:
#[serde(default)]
pub bookmarks: Vec<PathBuf>,
```

### Actions

```rust
AddBookmark,                // Ctrl+B — add active pane cwd to bookmarks
OpenBookmarks,              // Ctrl+Shift+B — open bookmarks modal
BookmarkSelect(usize),      // navigate to bookmark at index
DeleteBookmark(usize),      // remove bookmark at index
BookmarkMoveUp,
BookmarkMoveDown,
CloseBookmarks,
```

### Keybindings

| Key | Context | Action |
|---|---|---|
| `Ctrl+B` | Pane | `AddBookmark` |
| `Ctrl+Shift+B` | Any | `OpenBookmarks` |
| `Up` / `Down` | Bookmarks modal | `BookmarkMoveUp/Down` |
| `Enter` | Bookmarks modal | `BookmarkSelect(selection)` |
| `Delete` | Bookmarks modal | `DeleteBookmark(selection)` |
| `Esc` | Bookmarks modal | `CloseBookmarks` |

### State handling

```rust
Action::AddBookmark => {
    let cwd = self.panes.active_pane().cwd.clone();
    if !self.config.config.bookmarks.contains(&cwd) {
        self.config.config.bookmarks.push(cwd);
        let _ = self.config.save(Path::new(&self.config_path));
        self.status_message = String::from("bookmark added");
    }
}
Action::BookmarkSelect(idx) => {
    if let Some(path) = self.config.config.bookmarks.get(*idx).cloned() {
        self.overlay.close_all();
        commands.push(Command::ScanPane {
            pane: self.panes.focus.into(),
            path,
        });
    }
}
```

### UI

Bookmarks modal: centred, shows numbered list of saved paths. Selected path highlighted. Bottom hint: `Enter=navigate  Del=remove  Esc=close`.

---

## Part B: Trash

### New dependency

```toml
trash = { version = "3", default-features = false }
```

The `trash` crate supports Windows (Recycle Bin), macOS (Trash), and Linux (freedesktop.org Trash). No additional system configuration needed.

### `FileOperation::Trash`

```rust
pub enum FileOperation {
    Copy { source: PathBuf, destination: PathBuf },
    CreateDirectory { path: PathBuf },
    CreateFile { path: PathBuf },
    Delete { path: PathBuf },       // kept for programmatic permanent delete
    Move { source: PathBuf, destination: PathBuf },
    Rename { source: PathBuf, destination: PathBuf },
    Trash { path: PathBuf },        // ← new: send to OS recycle bin
}
```

### `fs.rs` — `trash_path`

```rust
pub fn trash_path(path: &Path) -> Result<(), FileSystemError> {
    trash::delete(path).map_err(|e| FileSystemError::DeletePath {
        path: path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
    })
}
```

### Jobs — handle `FileOperation::Trash` in file-op worker

```rust
FileOperation::Trash { path } => crate::fs::trash_path(path),
```

### Prompt change

`OpenDeletePrompt` currently queues `FileOperation::Delete`. Change it to queue `FileOperation::Trash` instead. Keep `FileOperation::Delete` available for the `shift+delete` variant (permanent, no trash):

```
F8              → Trash (recoverable)
Shift+F8        → Delete (permanent, existing behaviour)
```

Add a new `Action::OpenPermanentDeletePrompt` for `Shift+F8`.

### Tests

```rust
#[test]
fn add_bookmark_persists_to_config() { ... }

#[test]
fn delete_bookmark_removes_from_config() { ... }

#[test]
fn trash_operation_is_dispatched_on_f8() { ... }

#[test]
fn permanent_delete_dispatched_on_shift_f8() { ... }
```

---

## Final verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
git commit -m "chore: Wave 5B complete — bookmarks + trash"
```
