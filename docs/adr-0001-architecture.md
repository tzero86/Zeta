# ADR-0001: Core Architecture For The Terminal File Manager

- Status: Accepted
- Date: 2026-03-22

## Context

This project is intended to become a Norton Commander-like terminal file explorer and lightweight editor.
The product goal is a dual-pane, keyboard-first TUI with modern polish, fast startup, and low steady-state resource usage.

The current repository is in bootstrap state and does not yet contain a Rust crate.
That makes this a good point to record architectural constraints before implementation details harden.

The design should optimize for:

- low CPU usage while idle and during navigation
- low RAM usage for common directory and editing workflows
- fast startup and quick first render
- reliable local filesystem operations
- simple deployment as a native binary
- maintainable code structure for a small team or strong solo developer

## Decision

We will build the application as a native Rust terminal program using a modular monolith architecture.

Primary technical choices:

- Language: Rust stable
- Terminal I/O: `crossterm`
- TUI rendering: `ratatui`
- Editor buffer: `ropey`
- Message passing: `crossbeam-channel` or `flume`
- Config serialization: `serde` + `toml`
- Errors: `thiserror` in modules, `anyhow` only at the application boundary

High-level runtime model:

- one UI thread owns terminal input, state transitions, and rendering
- a small bounded worker pool handles filesystem jobs and expensive background work
- side effects occur outside render code
- state changes are driven by explicit actions, commands, and events

## Architecture Overview

The application will start as a modular monolith with the following major modules:

- `app`: top-level event loop and orchestration
- `state`: canonical application state and reducers
- `action`: user intents and internal commands
- `ui`: layout, widgets, and rendering
- `pane`: pane navigation, selection, sorting, and history
- `fs`: filesystem abstraction and file operations
- `jobs`: background copy, move, delete, and scan tasks
- `preview`: preview logic for text, binary, and metadata
- `editor`: lightweight embedded text editor
- `config`: theme, keymap, and persisted preferences

Expected event flow:

1. terminal input or worker completion arrives as an event
2. event maps to one or more actions
3. reducers update canonical state
4. reducers emit commands for side effects when needed
5. workers execute commands and return result events
6. render runs only if state changed or the UI requires refresh

This model keeps behavior deterministic, testable, and performant.

## Why Rust

Rust is the preferred language because it offers the best balance of:

- low CPU and RAM overhead
- strong control over allocation behavior
- safe handling of complex file and editor state
- good support for shipping a portable native binary
- a mature enough ecosystem for TUIs and text buffers

Alternatives considered:

- Go: faster to prototype, but typically higher memory usage and less control over latency-sensitive allocation behavior
- Zig: promising for low-level performance, but not yet as productive or mature for a polished TUI/editor product
- C++: capable, but significantly higher maintenance and safety cost for a solo or small-team tool

## UI And Rendering Model

The UI should feel modern through clarity and responsiveness, not expensive effects.

Guidelines:

- dual-pane layout is the default interaction model
- keyboard-first navigation remains the primary UX
- command palette and focused dialogs add modern ergonomics
- progress UI must remain non-blocking
- rendering should be incremental or diff-based when possible
- visible rows only should be rendered for large directories

The render loop should avoid unnecessary work:

- redraw only when state changes
- recompute layout fully on resize, not every tick
- debounce progress and watch-driven refreshes
- avoid expensive preview or metadata work on the UI thread

## Filesystem Model

The initial product scope is local filesystem operations only.

The filesystem layer should:

- use `Path` and `PathBuf` rather than raw strings
- handle metadata and directory scanning lazily where practical
- support copy, move, rename, delete, mkdir, touch, and overwrite flows
- treat permissions and symlinks as first-class cases
- surface failures as recoverable user-facing errors

The initial abstraction may define a filesystem trait, but v1 should only ship with a local filesystem implementation.
Remote backends and plugins are intentionally deferred.

## Editor Scope

The embedded editor is important, but it is not the first milestone.

The editor should begin as a lightweight text editor focused on:

- open and save
- dirty tracking
- search
- basic cursor movement and selection
- large file safeguards

The editor should not initially attempt to become a full IDE or Vim-class editor.
Heavy syntax highlighting, LSP integration, and rich editor plugins are explicitly out of scope for early versions.

## Concurrency Model

We will avoid a runtime-wide async architecture in the initial implementation.

Instead:

- UI state and rendering remain on one thread
- blocking filesystem operations run on a small bounded worker pool
- channels transport job progress and completion events back to the UI thread
- worker queues must be bounded to avoid unbounded memory growth

This keeps the architecture simple and predictable while still preventing the UI from blocking on slow file operations.

## Testing Strategy

We will bias toward tests that lock down behavior at the right layer:

- unit tests for reducers, sorting, filtering, selection, and path logic
- integration tests for filesystem behavior using temp directories
- snapshot tests only for stable render outputs
- benchmarks for startup, scrolling, and large directory scans before major optimizations

Cross-platform coverage matters because path handling, permissions, and file watching differ between operating systems.

## Consequences

Positive outcomes:

- low-overhead architecture from the start
- clean boundaries between state, rendering, and side effects
- safer file operations and editor logic
- portable single-binary distribution model
- room to add features later without rewriting the core loop

Tradeoffs:

- Rust raises implementation complexity compared with Go
- editor work remains a substantial complexity risk
- careful discipline is required to avoid slowly introducing async or plugin complexity too early

## What We Intentionally Avoid In Early Versions

- plugin systems
- remote filesystem support
- archive mounting
- rich syntax highlighting
- heavy async runtime adoption
- generalized extension APIs before stable internal boundaries exist

## Initial Milestones

1. bootstrap Rust crate and terminal runtime
2. implement event loop, canonical state, and dual-pane navigation
3. add filesystem service and basic file operations
4. add jobs, progress UI, and error handling
5. add preview flows and command palette
6. add embedded editor for small and medium text files
7. harden performance on large directories and large files

## Follow-Up

If architecture or stack decisions change materially, add a new ADR rather than silently rewriting this one.
