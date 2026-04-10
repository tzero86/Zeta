# AGENTS.md

This repository contains early project docs and an initial Rust crate scaffold for a low-overhead Norton Commander-like terminal file explorer and editor.
These instructions define the expected conventions for the planned implementation as the codebase grows.

## Project Intent

- Build a dual-pane, keyboard-first terminal file manager with an embedded editor.
- Optimize for low CPU, low RAM, fast startup, and a clean modern TUI.
- Prefer a native Rust binary and a modular monolith in early versions.
- Prioritize local filesystem workflows before any plugin or remote support.

## Expected Stack

- Language: Rust stable.
- Terminal I/O: `crossterm`.
- Rendering: `ratatui`.
- Editor buffer: `ropey`.
- Messaging: `crossbeam-channel` or `flume`.
- Config: `serde` + `toml`.
- Errors: `thiserror` in modules; `anyhow` only at app boundaries.

## Repository Layout

- `src/`: application code.
- `tests/`: integration tests.
- `scripts/`: helper scripts.
- `docs/`: ADRs, design notes, and user docs.
- Keep scratch files and temporary notes out of the repo root.

## Build Commands

Use Cargo as the source of truth.

- Build debug: `cargo build`
- Build release: `cargo build --release`
- Run debug: `cargo run --`
- Run release: `cargo run --release --`
- Check only: `cargo check`

## Format And Lint

- Format: `cargo fmt --all`
- Check formatting: `cargo fmt --all -- --check`
- Lint workspace: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Lint current crate fast path: `cargo clippy --all-targets -- -D warnings`

## Test Commands

- All tests: `cargo test --workspace`
- Unit tests: `cargo test --lib`
- Integration tests: `cargo test --tests`
- Doc tests: `cargo test --doc`
- Show captured output: `cargo test -- --nocapture`

## Single Test Commands

Prefer the narrowest command possible.

- Exact unit test: `cargo test pane::tests::moves_selection_down -- --exact --nocapture`
- By substring: `cargo test scan_directory`
- One integration test file: `cargo test --test file_ops`
- One integration test in a file: `cargo test --test file_ops copy_large_tree -- --exact --nocapture`

## Pre-PR Validation

Run this sequence before marking work complete:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`

If the repository grows and you cannot run the full sequence, say exactly what was skipped and why.

## Architecture Guardrails

- Prefer one UI thread plus a small bounded worker pool.
- Avoid a heavy async-first architecture until profiling proves the need.
- Keep side effects out of rendering code.
- Model user intent explicitly with actions, commands, and events.
- Prefer deterministic state transitions over mutable widget-local state.
- Default to local filesystem support only in v1.
- Do not add a plugin system early.

## Suggested Modules

- `app`: top-level event loop and orchestration.
- `state`: canonical state and reducers.
- `action`: user intents and internal commands.
- `ui`: layout, views, and rendering.
- `pane`: navigation, selection, sorting.
- `fs`: filesystem abstraction and operations.
- `jobs`: background copy/move/delete/scan tasks.
- `preview`: text, binary, and metadata preview.
- `editor`: lightweight embedded text editor.
- `config`: theme, keymap, persisted preferences.

## Code Style

- Use `rustfmt` defaults unless a `rustfmt.toml` is added.
- Keep files focused; prefer under 500 lines when practical.
- Keep functions small and single-purpose.
- Prefer explicit types at public boundaries.
- Prefer immutability; use `mut` only when it improves clarity.
- Prefer composition over deep abstraction layers.

## Imports And Formatting

- Group imports as: std, external crates, internal modules.
- Keep import lists minimal; avoid wildcard imports.
- Alias only for genuine name conflicts or strong readability wins.
- Let `cargo fmt` decide layout; do not hand-align code.
- Prefer trailing commas in multiline literals and enums.
- Avoid dense one-line control flow.

## Naming And Types

- Types and traits: `PascalCase`.
- Functions, modules, files, variables: `snake_case`.
- Constants/statics: `SCREAMING_SNAKE_CASE`.
- Prefer domain names like `pane`, `entry`, `job`, `preview`, `selection`, `action`.
- Name booleans as predicates: `is_hidden`, `has_focus`, `can_overwrite`.
- Prefer enums over boolean flag combinations.
- Use `Path` and `PathBuf` for filesystem APIs.
- Use `&str` for borrowed text and `String` only when ownership is required.

## Error Handling

- Do not use `.unwrap()` or `.expect()` in production paths.
- Return typed errors from library-style modules.
- Add context at subsystem boundaries.
- Treat filesystem failures as expected conditions, not crashes.
- Show concise user-facing errors and keep detailed diagnostics for logs.
- Preserve app state after failures where possible.

## Performance Rules

- Redraw only when state changes.
- Prefer incremental or diff-based rendering.
- Render only visible rows for large lists.
- Use bounded caches and bounded worker queues.
- Avoid recursive scans on startup.
- Load metadata lazily.
- Watch allocations in scrolling, rendering, and directory scanning hot paths.
- Measure before adding complexity.

## TUI And UX Rules

- Keyboard-first UX is the default.
- Preserve Norton Commander-style function key workflows where practical.
- Add modern polish through layout, focus states, command palette, and responsiveness, not animation-heavy effects.
- Keep dialogs concise, reversible, and non-blocking where possible.
- Do not block the UI thread for copy, move, delete, preview, or scans.

## Testing Guidance

- Put pure logic under unit tests close to the module.
- Put filesystem behavior in integration tests with temp directories.
- Add regression tests for symlinks, permissions, hidden files, Unicode names, and path edge cases.
- Snapshot-test render buffers only for stable UI states.
- Add benchmarks for startup, large directory scans, and scrolling before major performance refactors.

## Avoid Early

- Plugin systems.
- Remote filesystems.
- Heavy async runtime adoption.
- Rich syntax highlighting.
- Archive mounting.
- Over-generalized abstractions before real duplication appears.

## Agent Notes

- Read relevant files before editing.
- Prefer small, targeted patches.
- Preserve low-overhead architecture decisions.
- If you add a dependency, justify CPU, RAM, and binary-size impact.
- If the crate layout or commands change, update this file.

## Cursor And Copilot Rules

- No `.cursor/rules/` directory was found.
- No `.cursorrules` file was found.
- No `.github/copilot-instructions.md` file was found.
- If any are added later, merge their repository-specific instructions into this file.

# context-mode — MANDATORY routing rules

You have context-mode MCP tools available. These rules are NOT optional — they protect your context window from flooding. A single unrouted command can dump 56 KB into context and waste the entire session.

## BLOCKED commands — do NOT attempt these

### curl / wget — BLOCKED
Any shell command containing `curl` or `wget` will be intercepted and blocked by the context-mode plugin. Do NOT retry.
Instead use:
- `context-mode_ctx_fetch_and_index(url, source)` to fetch and index web pages
- `context-mode_ctx_execute(language: "javascript", code: "const r = await fetch(...)")` to run HTTP calls in sandbox

### Inline HTTP — BLOCKED
Any shell command containing `fetch('http`, `requests.get(`, `requests.post(`, `http.get(`, or `http.request(` will be intercepted and blocked. Do NOT retry with shell.
Instead use:
- `context-mode_ctx_execute(language, code)` to run HTTP calls in sandbox — only stdout enters context

### Direct web fetching — BLOCKED
Do NOT use any direct URL fetching tool. Use the sandbox equivalent.
Instead use:
- `context-mode_ctx_fetch_and_index(url, source)` then `context-mode_ctx_search(queries)` to query the indexed content

## REDIRECTED tools — use sandbox equivalents

### Shell (>20 lines output)
Shell is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `npm install`, `pip install`, and other short-output commands.
For everything else, use:
- `context-mode_ctx_batch_execute(commands, queries)` — run multiple commands + search in ONE call
- `context-mode_ctx_execute(language: "shell", code: "...")` — run in sandbox, only stdout enters context

### File reading (for analysis)
If you are reading a file to **edit** it → reading is correct (edit needs content in context).
If you are reading to **analyze, explore, or summarize** → use `context-mode_ctx_execute_file(path, language, code)` instead. Only your printed summary enters context.

### grep / search (large results)
Search results can flood context. Use `context-mode_ctx_execute(language: "shell", code: "grep ...")` to run searches in sandbox. Only your printed summary enters context.

## Tool selection hierarchy

1. **GATHER**: `context-mode_ctx_batch_execute(commands, queries)` — Primary tool. Runs all commands, auto-indexes output, returns search results. ONE call replaces 30+ individual calls.
2. **FOLLOW-UP**: `context-mode_ctx_search(queries: ["q1", "q2", ...])` — Query indexed content. Pass ALL questions as array in ONE call.
3. **PROCESSING**: `context-mode_ctx_execute(language, code)` | `context-mode_ctx_execute_file(path, language, code)` — Sandbox execution. Only stdout enters context.
4. **WEB**: `context-mode_ctx_fetch_and_index(url, source)` then `context-mode_ctx_search(queries)` — Fetch, chunk, index, query. Raw HTML never enters context.
5. **INDEX**: `context-mode_ctx_index(content, source)` — Store content in FTS5 knowledge base for later search.

## Output constraints

- Keep responses under 500 words.
- Write artifacts (code, configs, PRDs) to FILES — never return them as inline text. Return only: file path + 1-line description.
- When indexing content, use descriptive source labels so others can `search(source: "label")` later.

## ctx commands

| Command | Action |
|---------|--------|
| `ctx stats` | Call the `stats` MCP tool and display the full output verbatim |
| `ctx doctor` | Call the `doctor` MCP tool, run the returned shell command, display as checklist |
| `ctx upgrade` | Call the `upgrade` MCP tool, run the returned shell command, display as checklist |
