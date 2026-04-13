# Zeta File Manager - Embedded Terminal (Wave 8A)

**Goal:** Implement a fully embedded terminal emulator within Zeta. Instead of dropping out to a shell (Wave 5C), this feature will provide a dedicated terminal panel or pane that renders alongside the file manager, similar to integrated terminals in modern IDEs.

## Phase 1: Terminal Backend Integration
- Integrate a terminal emulation crate (e.g., `alacritty_terminal`, `vt100`, or `portable-pty`).
- Create a PTY (pseudo-terminal) spawn mechanism to run the user's default shell (`$SHELL` or `cmd.exe`/`powershell.exe`).
- Connect the PTY's read/write streams to a dedicated background worker (`TerminalWorker`) to ensure the main UI thread is never blocked by shell output.

## Phase 2: UI and Rendering
- Design a new `TerminalState` within `AppState` to hold the grid of cells, cursor position, scrollback buffer, and color palettes.
- Add a new rendering module `src/ui/terminal.rs` to map the terminal emulator's grid state to ratatui's `Buffer`.
- Allow the terminal to be toggled via a global keybinding (e.g., `Ctrl+\` or `F2`).
- Support split-view rendering (e.g., terminal taking the bottom 30% of the screen) or replacing one of the active panes.

## Phase 3: Input Routing
- Add a `FocusLayer::Terminal` to route all keystrokes directly to the PTY.
- Ensure terminal escape sequences, control codes, and special keys (like `Ctrl+C`, `Arrows`, `Alt`) are correctly translated and forwarded.
- Provide a dedicated keybinding (e.g., `Ctrl+\`) to break focus out of the terminal and back to the file manager navigation.

## Acceptance Criteria
- A fully functional shell session can be opened within the Zeta UI.
- Terminal output renders correctly, including basic ANSI colors and cursor positioning.
- Keystrokes are accurately routed to the shell, allowing interactive commands like `vim` or `htop` to run (if supported by the emulation library).
- The application remains responsive while the terminal is producing heavy output.