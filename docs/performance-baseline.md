# Performance Baseline

## Initial Targets

- startup target: under 150 ms on a warm local development machine for the initial MVP shell
- directory scan target: under 50 ms for small to medium local directories used during normal development
- idle redraw target: zero redraws unless state changes or the terminal is resized

## Current Instrumentation

- startup timing is captured during `App::bootstrap` and shown in the status bar as `startup:<n>ms`
- directory scan timing is measured in the background scan worker and shown after refresh as `scan:<n>ms`
- redraw count is tracked in application state and shown in the status bar as `draws:<n>`

## How To Verify

Run the application:

```bash
cargo run --
```

Use these checks while the app is open:

- observe the first status line after startup for startup timing
- press `r` on either pane to refresh and capture scan timing
- leave the UI idle and confirm the redraw count stays stable

## Notes

- This baseline is intentionally simple and low-overhead.
- Add benchmark automation later only if manual timing is not enough for regression tracking.
