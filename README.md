# Zeta

Zeta is a keyboard-first terminal file manager and lightweight editor written in Rust.
It aims for a classic Norton Commander workflow with a cleaner modern TUI, low overhead, and fast local filesystem operations.

## Current Status

This project is in active early development.

Implemented so far:

- dual-pane file browser
- side-by-side and stacked pane layouts
- embedded text editor with save/discard flow
- top menu bar, prompts, and dialogs
- theme switching
- Unicode icons by default with ASCII fallback in config
- create, rename, delete, copy, and move flows
- background jobs for scans and file operations
- GitHub Actions builds for Linux and Windows artifacts

## Tech Stack

- Rust stable
- `crossterm` for terminal I/O
- `ratatui` for rendering
- `ropey` for editor buffers
- `crossbeam-channel` for background job messaging
- `serde` + `toml` for config

## Run Locally

```bash
cargo run --
```

Useful commands:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Project Layout

- `src/app.rs` - event loop and orchestration
- `src/state.rs` - canonical app state and reducers
- `src/ui.rs` - rendering and layout
- `src/fs.rs` - filesystem operations
- `src/jobs.rs` - background workers
- `src/editor.rs` - embedded editor logic
- `docs/adr-0001-architecture.md` - architecture decision record

Config note:

```toml
# icon_mode = "unicode"  # default
# icon_mode = "ascii"    # safe fallback for limited terminals
```

## Direction

Near-term priorities:

- mature filesystem UX around collisions and overwrite flows
- keep file operations non-blocking
- improve test coverage for filesystem and render behavior
- continue refining the commander-style interaction model

## License

Currently unlicensed unless a project license file is added explicitly.
