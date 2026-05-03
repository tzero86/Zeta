# Zeta TUI Tests

Integration tests for Zeta's terminal UI using [**@microsoft/tui-test**](https://github.com/microsoft/tui-test) — a framework that spawns the real binary in a PTY and drives it with keyboard input, then asserts on rendered terminal output.

## Prerequisites

- Node.js 18+ (22 recommended)
- A built `zeta` binary (debug or release)
- A POSIX-compatible shell — the test isolation wrapper (`zeta-wrapper.sh`) uses `bash` and `mktemp`.
  On Windows, run tests inside **WSL** or **Git Bash**.

## Setup

```bash
# 1. Build the Zeta binary
cargo build          # debug (default)
# or
cargo build --release

# 2. Install test dependencies
cd tui-tests
npm install
```

## Running Tests

```bash
# Run all TUI tests (from tui-tests/ directory)
npm test

# Run a specific test file
npm run test:smoke
npm run test:navigation
npm run test:cheatsheet
npm run test:editor

# Run with trace capture (writes replays to tui-traces/)
npm run test:trace

# Use a release build
ZETA_BIN=../target/release/zeta npm test
```

### Running from the fixtures directory

Several tests (especially `editor.test.ts`) work best when Zeta opens in a directory that contains known files. Run from `tui-tests/fixtures/` for predictable results:

```bash
cd tui-tests/fixtures
npx tui-test ..
```

Alternatively, point Zeta at the fixtures directory by setting your shell's CWD:

```bash
cd tui-tests
ZETA_BIN=../target/debug/zeta npx tui-test
```

> **Note:** Zeta uses its working directory as the starting path, so wherever you launch the tests from, Zeta will open there.

## Test Files

| File | What it covers |
|------|----------------|
| `smoke.test.ts` | App starts, dual-pane UI renders, F-key bar visible, Ctrl+Q quits |
| `navigation.test.ts` | Tab pane switch, arrow key movement, Backspace up-dir, Shift+F10 context menu |
| `cheatsheet.test.ts` | `?` opens overlay, correct section titles, Esc closes, toggle behavior |
| `editor.test.ts` | F4 opens editor, Esc returns to file manager, editor cheatsheet |
| `filter.test.ts` | Sort cycling (`s`), pane filter (`/`), file marks (`Space`) |
| `files.test.ts` | F5 copy dialog, F7 new dir dialog, F8 delete dialog, `r` rename |
| `overlays.test.ts` | Ctrl+P file finder, Shift+P command palette, F1 help, Ctrl+O settings, bookmarks |
| `workspaces.test.ts` | Alt+1/2/3 workspace tab switching |
| `preview.test.ts` | F3 opens preview panel, F3 twice closes it |

## Fixtures

`fixtures/` contains a small directory tree used by tests that need real files:

```
fixtures/
  README.md
  src/
    hello.rs
  documents/
    note1.txt
    note2.txt
```

## Traces

When tests fail (or with `--trace`), tui-test writes replay files to `tui-traces/`. Replay them with:

```bash
npm run show-trace
```

## CI Integration

Add to your CI workflow:

```yaml
- name: Build Zeta
  run: cargo build

- name: Run TUI tests
  working-directory: tui-tests
  run: |
    npm ci
    npm test
  env:
    CI: true
```

The `CI=true` environment variable enables automatic retries (up to 2) and trace capture on failure.
