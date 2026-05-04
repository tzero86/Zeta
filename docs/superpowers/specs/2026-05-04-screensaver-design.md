# Screensaver — Design Spec

Date: 2026-05-04  
Status: Draft  
Branch: `feat/site-redesign`

## Summary

An atmospheric weather-themed ASCII screensaver that activates when Zeta detects terminal idle time. Renders drifting particles, wind streaks, sporadic rain, and a pulsing Zeta logo using direct ratatui buffer cell writes — no new dependencies.

## Motivation

Terminal file managers spend most of their time waiting for the user. A screensaver turns idle terminal time into a visual experience that reinforces the project's identity without burning meaningful CPU or RAM (target: <1% CPU, <1 MB animation state).

## Architecture

### New module: `src/screensaver/`

```
src/screensaver/
  mod.rs   — all state + rendering logic (~350 lines)
```

No public API surface beyond `ScreensaverState` and a `ScreensaverWidget`.

### Key types

```rust
pub struct ScreensaverState {
    pub active: bool,
    pub enabled: bool,
    pub timeout_secs: u64,
    last_interaction: Instant,
    last_frame: Instant,
    particles: Vec<Particle>,
    logo_bounds: Rect,
    wind_phase: f64,
    gust_timer: f64,
    rain_burst_timer: f64,
    frame_counter: u64,
}

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    ch: char,
    layer: u8,         // 0=background, 1=midground, 2=foreground
    brightness: u8,
}
```

### Files touched (9 existing, 1 new)

| File | Change |
|------|--------|
| `src/screensaver/mod.rs` | NEW — screensaver state, particle system, wind/rain simulation, render widget |
| `src/app.rs` | Add `last_interaction` field, idle detection in `process_next_event()`, animation frame tick |
| `src/state/mod.rs` | Add `screensaver: ScreensaverState` to `AppState`, `DismissScreensaver` / `ActivateScreensaver` action handlers |
| `src/state/types.rs` | Add `FocusLayer::Screensaver` variant |
| `src/action.rs` | Add `DismissScreensaver`, `ActivateScreensaver` actions; add routing branch in `route_key_event()` |
| `src/config.rs` | Add `screensaver_timeout_secs: u64` (default 300), `screensaver_enabled: bool` (default true) |
| `src/state/settings.rs` | Add `ScreensaverTimeout(u64)` and `ScreensaverEnabled(bool)` to `SettingsField` |
| `src/ui/mod.rs` | Add screensaver render call as final overlay layer |
| `src/lib.rs` | `mod screensaver` |
| `src/ui/settings.rs` | Add screensaver fields display in Panels tab |

## Animation System

### Particle layers

Three independent layers with distinct behavior:

| Layer | Count | Characters | Behavior | Color |
|-------|-------|------------|----------|-------|
| Background | ~40 | `.` `·` ` ` | Slow drift, no direction preference | `#2a3a4a` (dim) |
| Midground | ~25 | `*` `o` `~` `'` `"` `,` | Diagonal drift left-to-right, gusts boost speed | `#5a7a8a` → `#8ab0c0` |
| Foreground | ~15 | `╌` `╍` `~` `─` | Horizontal wisps, group into gusts | `#7a9aaa` (brighter) |

### Wind dynamics

- Base wind speed oscillates sinusoidally (period ~20s): calm → breeze → calm
- Gusts trigger every 8-20s (random): particles 2-3x speed for ~2s
- During gusts, midground and foreground layers add extra streaks
- Direction: primarily left-to-right; occasional reversal (rare, ~5% chance per cycle)

### Rain

- ~15 vertical streaks using `│` `┊` `⋮`
- Burst-based: rain for 3-8s, then dry for 5-15s
- Slight rightward angle as particles fall
- Intensity varies: light (3-5 streaks) ↔ steady (10-15 streaks)

### Zeta logo

- `[Z]eta` centered ~1/3 from top of content area
- Breathing glow: brightness oscillates 180-255 over ~4s
- Accent blue `#82aaff` with subtle edge fade
- Faint box border using `│─┌┐└┘` characters

## Configuration

### Config file (`config.toml`)

```toml
screensaver_timeout_secs = 300   # 5 minutes, 0 to disable timer
screensaver_enabled = true       # master toggle
```

### State machine

```
IDLE        --[elapsed > timeout]--> ACTIVE
ACTIVE      --[any key/mouse]------> IDLE
```

- `last_interaction` updated on every `Event::Key`, `Event::Mouse`, `Event::Resize`
- Timer checked in the idle branch of `process_next_event()`

### Manual trigger

- Command palette entry: `Activate Screensaver` (maps to `Action::ActivateScreensaver`)
- Sets `active = true` immediately, bypassing timer

### Settings panel

- Panels tab gets two new fields:
  - `Screensaver [on/off]` — toggle enabled
  - `Screensaver timeout [300s]` — adjust in 30s increments via up/down

## Integration Points

### Event loop (`src/app.rs`)

On idle tick (no event received within 16ms poll window):
1. If screensaver active and 83ms since last frame → redraw
2. If screensaver inactive and enabled and timer expired → activate

### Key routing (`src/action.rs`)

```rust
FocusLayer::Screensaver => Some(Action::DismissScreensaver),
```

All key and mouse events while in screensaver focus layer immediately dismiss. No event inspection needed.

### Render pipeline (`src/ui/mod.rs`)

```rust
if state.screensaver.active {
    let ss_area = areas[1]; // main content area
    frame.render_widget(screensaver::ScreensaverWidget, ss_area);
}
```

Rendered last, on top of all other content. The widget implements `ratatui::widgets::Widget` and writes directly to `Buffer` cells.

## Edge Cases

- **Timeout = 0**: Screensaver never activates via timer; manual trigger still works. Activation logic checks `timeout_secs > 0` before reading elapsed time.
- **Enabled = false**: No activation at all, timer never checked.
- **Terminal resize during screensaver**: Next frame re-evaluates particle bounds. Particles outside new bounds wrap to opposite edge.
- **Very small terminals (< 40 cols)**: Hide particles, render only logo (centered, scaled down).
- **Manual trigger while already active**: No-op (already active).

## Non-Goals

- No audio effects
- No configuration beyond timeout and enable toggle
- No external rendering engine or shader language
- No plugin system integration

## Future (explicitly deferred)

- Additional screensaver themes (different visual styles)
- Per-workspace screensaver preferences
- Screensaver password / lock on dismiss
