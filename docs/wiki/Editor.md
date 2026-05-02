# Editor

Zeta includes a full-featured embedded text editor so you never have to leave the file manager to make a quick change. Open any file with `F4` from the file pane.

---

## Opening and Closing the Editor

| Key | Action |
|---|---|
| `F4` | Open selected file in editor |
| `Ctrl+S` | Save |
| `Ctrl+Q` | Close editor (prompts if unsaved) |

If the file has unsaved changes, Zeta shows a **save / discard** confirmation dialog before closing. You cannot accidentally lose work.

---

## Editing

- Full Unicode input support
- Configurable **tab width** (`tab_width` in config) — tabs insert the configured number of spaces
- Optional **word wrap** (`word_wrap` in config) — wraps long lines at the viewport width
- Dirty-state indicator in the editor title bar when unsaved changes exist

### Undo / Redo

Zeta uses a **delta-based undo history** backed by `ropey::Rope` (B-tree). Operations are O(log n) even on large files.

| Key | Action |
|---|---|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` or `Ctrl+Shift+Z` | Redo |

---

## Text Selection and Clipboard

| Key | Action |
|---|---|
| `Shift+Arrow` | Extend selection |
| `Ctrl+A` | Select all |
| `Ctrl+C` | Copy selection |
| `Ctrl+X` | Cut selection |
| `Ctrl+V` | Paste |

Typing while a selection is active replaces the selected text.

---

## Search

Press `Ctrl+F` to open the in-editor search bar.

- **Match count** is shown while searching (`3 of 17`)
- **Wrap-around**: search loops from end back to beginning automatically
- `Enter` / `n` advances to the next match; `Shift+Enter` / `N` goes to the previous match
- `Esc` closes search and returns focus to the editor

Matches are highlighted across the entire visible buffer.

---

## Syntax Highlighting

Zeta uses [`syntect`](https://crates.io/crates/syntect) for syntax highlighting. Highlighting is computed once per edit (keyed by edit version) — never per frame — so it is fast even on large files.

Supported languages include all common programming languages: Rust, Python, JavaScript/TypeScript, Go, C/C++, Shell, TOML, YAML, JSON, Markdown, and many more.

---

## Markdown Live Preview

When editing a `.md` file, the editor pane splits vertically:

- **Left**: the editor as normal
- **Right**: a live rendered preview of the Markdown

The preview updates on every keystroke with zero latency — no background job, no delay.

**Supported Markdown elements:**
- Headings (H1–H6)
- Bold and italic text
- Inline code and fenced code blocks
- Bullet and ordered lists
- Blockquotes
- Horizontal rules

---

## Editor Configuration

Add these keys to your `config.toml` under the `[editor]` section:

```toml
[editor]
tab_width = 4
word_wrap = false
```

Access these settings at runtime in the settings panel (`Ctrl+O`).

---

*See also: [Key Bindings](Key-Bindings) · [Configuration](Configuration) · [File Management](File-Management)*
