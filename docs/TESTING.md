# Testing Guide for Zeta

Zeta has a comprehensive testing strategy covering unit tests, integration tests, and end-to-end (E2E) tests.

## Test Organization

- **Unit Tests**: Located in `src/` modules, test individual functions and types.
- **Integration Tests**: Located in `tests/`, test filesystem operations, terminal I/O, and SSH workflows.
- **E2E Tests**: Located in `tests/e2e_integration.rs`, test keyboard workflows and screen interaction.

## Running Tests

### All Tests
```bash
cargo test --workspace
```

### Unit Tests Only
```bash
cargo test --lib
```

### Integration Tests
```bash
cargo test --tests
```

### E2E Tests Only
```bash
cargo test --tests e2e_ --test-threads=1
```

### Single Test
```bash
cargo test --lib pane::tests::moves_selection_down -- --exact --nocapture
```

## End-to-End (E2E) Testing

### Overview

E2E tests verify keyboard-driven workflows using the `ZetaE2eInstance` harness. Tests spawn a Zeta process, send keyboard input, and verify behavior without mocking the app logic.

### Architecture

The E2E harness (`src/testing/e2e.rs`) provides:

- **Process Management**: Spawns and monitors Zeta in release mode
- **Keyboard Input**: Translates `crossterm::KeyCode` to terminal escape sequences
- **Screen Verification**: Mock helpers for asserting app state (future: full PTY output parsing)
- **Lifecycle**: Automatic cleanup on test completion

### Writing E2E Tests

```rust
use std::time::Duration;
use crossterm::event::KeyCode;
use zeta::testing::ZetaE2eInstance;

#[test]
fn e2e_my_workflow() {
    // 1. Spawn the app
    let mut zeta = ZetaE2eInstance::spawn().expect("spawn");
    
    // 2. Verify startup
    assert!(
        zeta.wait_for_text("Zeta", Duration::from_secs(2))
            .expect("screen check"),
        "App should display 'Zeta' on startup"
    );

    // 3. Send keyboard input
    zeta.send_key(KeyCode::Down).expect("send key");
    zeta.wait_for_render().expect("wait render");

    // 4. Verify result
    let screen = zeta.capture_screen().expect("capture");
    assert!(!screen.is_empty(), "Screen should render without crash");
    
    // 5. Cleanup (automatic on drop)
}
```

### Supported Key Codes

The harness translates these `crossterm` key codes to terminal escape sequences:

- **Characters**: `KeyCode::Char('a')` → `'a'` (ASCII)
- **Navigation**: `Up`, `Down`, `Left`, `Right` → arrow key sequences
- **Page**: `Home`, `End`, `PageUp`, `PageDown` → navigation sequences
- **Editing**: `Enter`, `Backspace`, `Tab`, `Esc`, `Delete`
- **Functions**: `F(1)` through `F(10)` → F1-F10 sequences

### API Reference

#### `ZetaE2eInstance::spawn() -> Result<Self>`
Spawn a new Zeta instance. Returns Ok even if the process fails to start (tests should verify startup with `wait_for_text`).

#### `send_key(code: KeyCode) -> Result<()>`
Send a single key to the app. Includes a 50ms delay for processing.

#### `send_text(text: &str) -> Result<()>`
Send raw text input (e.g., filter terms, file names).

#### `wait_for_text(text: &str, timeout: Duration) -> Result<bool>`
Poll for text to appear on screen. Returns true if found, false on timeout.

#### `wait_for_render() -> Result<()>`
Wait 200ms for a render cycle to complete.

#### `capture_screen() -> Result<Vec<String>>`
Get current screen content as lines. Currently a mock; future versions will parse PTY output via vt100.

#### `screen_contains(text: &str) -> Result<bool>`
Check if text appears on screen. Currently always returns true (mock); future implementation will parse real output.

#### `shutdown() -> Result<()>`
Gracefully quit the app and clean up. Called automatically on drop.

### Current Limitations

1. **Screen Parsing**: `capture_screen()` and `screen_contains()` are mocks. Tests verify that the app doesn't crash, not the exact output.
2. **PTY Integration**: Keyboard input is sent, but output is not yet fully captured and parsed. This is a foundation for future expansion.
3. **No Mouse**: E2E tests use keyboard-only input (Zeta is keyboard-first by design).
4. **Execution Speed**: Tests spawn the full app in release mode (~1-2 seconds each). Keep test count reasonable.

### Future Enhancements

- **Full PTY Output Parsing**: Integrate `vt100` parser to capture and verify exact screen content
- **Terminal Assertions**: Helper methods like `assert_contains()`, `assert_position()` for screen location matching
- **Workflow Templates**: Reusable patterns for common tasks (file copy, move, delete)
- **Performance Benchmarks**: Measure startup time, directory scan speed, render latency

## Performance Considerations

- Unit tests should complete in < 1 second total
- Integration tests should complete in < 5 seconds total  
- E2E tests should complete in < 10 seconds total (run with `--test-threads=1` to avoid PTY conflicts)
- Do not add tests that require external resources (API calls, SSH servers) unless in a `#[ignore]` block

## Pre-PR Validation

Run this sequence before opening a pull request:

```bash
# 1. Format code
cargo fmt --all -- --check

# 2. Lint warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. All tests (unit + integration + E2E)
cargo test --workspace
```

If you cannot run the full sequence due to environment constraints, document what was skipped.

## Troubleshooting

### Tests hang or timeout
- Check that no Zeta processes are still running: `pkill -f 'cargo run' || true`
- Run tests with `--test-threads=1` to avoid PTY resource conflicts
- Increase timeout in test with `Duration::from_secs(5)` instead of default

### "Broken pipe" errors
- This indicates stdin/stdout closed unexpectedly. Ensure the app doesn't exit immediately after `spawn()`.
- Mock implementations allow tests to pass even if the process fails.

### E2E tests fail with "command not found: cargo"
- Ensure `cargo` is in `$PATH` and the test is run from the repository root.
- On CI, pre-build the app: `cargo build --release` before running tests.

## Test Patterns

### Verify app starts
```rust
let mut zeta = ZetaE2eInstance::spawn().expect("spawn");
assert!(zeta.wait_for_text("Zeta", Duration::from_secs(2)).expect("check"));
```

### Send multiple keys
```rust
for _ in 0..3 {
    zeta.send_key(KeyCode::Down).expect("key");
    zeta.wait_for_render().expect("render");
}
```

### Input text and confirm
```rust
zeta.send_text("test.txt").expect("text");
zeta.send_key(KeyCode::Enter).expect("enter");
zeta.wait_for_render().expect("render");
```

### Quit cleanly
```rust
zeta.send_key(KeyCode::Char('q')).expect("quit");
zeta.shutdown().expect("shutdown");
```

## References

- Zeta Architecture: `docs/ARCHITECTURE.md` (future)
- Keyboard Handling: `src/action.rs::route_key_event`
- Event System: `src/event.rs`
- E2E Harness: `src/testing/e2e.rs`
