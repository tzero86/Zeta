# Editor Replacement Feasibility — 2026-05-05

## Context

Zeta embeds a custom text editor (~2 500 lines across `src/editor.rs`,
`src/state/editor_state.rs`, and `src/ui/editor.rs`).  The custom editor
uses `ropey` for the rope buffer and has accumulated several correctness
bugs: selection skipping empty lines, CRLF handling, absent mouse
support, and no undo history.  This document evaluates whether replacing
the custom editor with an existing Rust TUI editor crate is worth the
cost and risk.

---

## Pain Points in the Current Editor

| Issue | Severity |
|---|---|
| Selection skips empty lines / CRLF lines (markdown tables, backtick blocks) | High |
| No mouse click / drag selection | High |
| No undo / redo | High |
| No multi-cursor or block selection | Medium |
| Syntax highlighting absent | Medium |
| ~2 500 lines of bespoke, growing code to maintain | Medium |

---

## Candidate Crates

### 1. `tui-textarea` (crates.io — `tui-textarea`)

| Attribute | Value |
|---|---|
| Dependency count | ~4 (ratatui, unicode-width, …) |
| Binary size delta | +~250 KB |
| RAM overhead | Minimal (Vec<String> buffer) |
| Undo / redo | Yes (built-in) |
| Mouse support | Yes |
| Multi-line selection | Yes |
| Syntax highlight | No (caller applies spans) |
| ratatui version | Tracks ratatui closely; supports 0.30 |
| Maintenance | Active; 0.7.x series as of 2026 |
| Integration effort | Medium — widget API, not a full app; caller drives key events |

**Pros:** Drop-in ratatui widget; immediate undo/redo; correct multi-line
selection; mouse; actively maintained; small dependency footprint.

**Cons:** No rope back-end (Vec<String>, fine for files < 10 MB); no
built-in syntax highlighting; no embedded LSP hooks; public API is
somewhat opinionated (cursor position is part of widget state, not
caller's state).

**Verdict: Recommended.** Solves every current pain point with the
least integration risk.

---

### 2. `edtui` (crates.io — `edtui`)

| Attribute | Value |
|---|---|
| Dependency count | ~6 |
| Binary size delta | +~400 KB |
| RAM overhead | Minimal |
| Undo / redo | Partial |
| Mouse support | No (as of 0.6) |
| Maintenance | Slower cadence; last release ~3 months old |
| Integration effort | Medium |

**Verdict: Lower priority.** Lacks mouse support and has a slower
maintenance pace than `tui-textarea`.  Revisit if `tui-textarea`
proves problematic.

---

### 3. `helix-core` (internal library of the Helix editor)

| Attribute | Value |
|---|---|
| Dependency count | 40+ |
| Binary size delta | +~8–12 MB |
| RAM overhead | ~15 MB cold |
| Undo / redo | Yes (full) |
| Mouse support | Yes |
| Syntax highlight | Yes (tree-sitter) |
| Maintenance | Active (helix project) |
| Integration effort | Very High — not a widget; no public stable API |

**Verdict: Not recommended for v1.**  The dependency surface and binary
cost contradict Zeta's low-overhead charter.  Suitable only if Zeta
later targets feature parity with a full programmer's editor.

---

## Recommendation

**Replace the custom editor with `tui-textarea` in a single focused
sprint.**

### Migration Plan Sketch

1. Add `tui-textarea = "0.7"` to `Cargo.toml`.
2. Create `src/editor/tui_textarea_adapter.rs` with a thin wrapper that
   exposes the same surface the rest of the app uses:
   - `open_file(path, content)` / `content() -> &str` / `is_modified()`
   - `apply(action: &Action) -> Vec<Command>`
   - `render(frame, rect, focused, theme)`
3. Replace `EditorBuffer` usages in `src/state/editor_state.rs` with the
   adapter.
4. Delete `src/editor.rs` (~1 664 lines) and the custom render code in
   `src/ui/editor.rs` (~312 lines); keep tests for adapter behavior.
5. Wire mouse click/drag through `tui-textarea`'s built-in mouse input.
6. Keep `ropey` in `Cargo.toml` only if any other module uses it;
   otherwise remove to reclaim ~200 KB from the binary.

### Effort Estimate

| Task | Rough Complexity |
|---|---|
| Add crate + adapter skeleton | Small |
| Action routing (keyboard + mouse) | Medium |
| Theme integration (custom spans for cursor/selection) | Small |
| Remove old editor code + tests | Small |
| Regression test pass | Medium |

**Total: 1–2 focused sessions.**

### Risks

- `tui-textarea`'s `TextArea` widget owns cursor state internally.  Some
  pieces of Zeta that currently read cursor position from `EditorBuffer`
  directly will need to go through the adapter.
- Markdown preview sync (`sync_markdown_preview_to_cursor`) must be
  re-implemented against the adapter's cursor API.
- The `ropey`-backed large-file path (> 10 MB) will be lost; acceptable
  for v1 since the current editor has no meaningful large-file handling
  either.

---

## Decision

Accept the recommendation to integrate `tui-textarea` as the editor
back-end.  Implementation work to be planned separately.  The current
bug fixes on `fix/editor-selection-and-ui-regressions` remain valuable:
they unblock users on the current editor while the replacement is
planned.
