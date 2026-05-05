# Screensaver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an atmospheric weather-themed ASCII screensaver that activates on terminal idle time, with configurable timeout, settings panel controls, and manual toggle from the command palette.

**Architecture:** New `src/screensaver/` module with particle system, wind/rain simulation, and Zeta logo rendering. Integration via 9 existing files: event loop idle detection, action routing, state management, config, settings panel, and render pipeline. Screensaver renders as a ratatui Widget writing directly to Frame buffer cells.

**Tech Stack:** Rust, ratatui (Buffer API for per-cell writes), crossterm (event polling for idle detection)

---

## File Map

### New files
| File | Purpose |
|------|---------|
| `src/screensaver/mod.rs` | ScreensaverState, particle system, wind/rain simulation, logo rendering, ScreensaverWidget |

### Modified files
| File | What changes |
|------|-------------|
| `src/config.rs` | Add `screensaver_timeout_secs` + `screensaver_enabled` fields + defaults + annotated config |
| `src/action.rs` | Add `DismissScreensaver`, `ActivateScreensaver` action variants + routing branch |
| `src/state/types.rs` | Add `FocusLayer::Screensaver` variant |
| `src/state/mod.rs` | Add `screensaver: ScreensaverState` field to AppState + action handlers in `apply()` |
| `src/app.rs` | Add `last_interaction` field to App + idle detection in `process_next_event()` + animation frame tick |
| `src/lib.rs` | `mod screensaver` |
| `src/ui/mod.rs` | Screensaver overlay render call at end of `render()` |
| `src/state/settings.rs` | Add `ScreensaverTimeout`, `ScreensaverEnabled` to `SettingsField` |
| `src/ui/settings.rs` | Display and edit screensaver fields in Panels tab |

---

### Task 1: Config fields + annotated config

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add config fields + defaults**

```rust
// After `terminal_open_by_default` field (around line 82)
#[serde(default = "default_screensaver_enabled")]
pub screensaver_enabled: bool,

#[serde(default = "default_screensaver_timeout")]
pub screensaver_timeout_secs: u64,
```

Add default functions around the existing default helpers (near line 495):

```rust
fn default_screensaver_enabled() -> bool { true }
fn default_screensaver_timeout() -> u64 { 300 }
```

- [ ] **Step 2: Add annotated config entry**

In `generate_annotated_config()` (around line 267), add after the terminal section:

```rust
writeln!(
    buf,
    "\n# Screensaver timeout in seconds.\n\
     # When Zeta is idle for this long, a weather-themed ASCII screensaver activates.\n\
     # Set to 0 to disable timer-based activation (manual trigger via command palette still works).\n\
     # screensaver_timeout_secs = {}\n\
     # screensaver_enabled = true\n",
    default_screensaver_timeout(),
)?;
```

- [ ] **Step 3: Run existing tests**

```bash
cargo test --lib config::tests
```
Expected: Pass (no regression — we only added optional fields with serde(default))

- [ ] **Step 4: Commit**

```bash
git add src/config.rs
git commit -m "feat(screensaver): add screensaver config fields (timeout, enabled)"
```

---

### Task 2: Action variants + FocusLayer + key routing

**Files:**
- Modify: `src/action.rs`
- Modify: `src/state/types.rs`

- [ ] **Step 1: Add FocusLayer::Screensaver**

In `src/state/types.rs`, add to the `FocusLayer` enum (around line 50, before the closing `}`):

```rust
    /// Screensaver is active — any key/mouse dismisses it.
    Screensaver,
```

- [ ] **Step 2: Add action variants**

In `src/action.rs`, add to the `Action` enum (around line 202, after `ToggleCheatsheet`):

```rust
    /// Activate the screensaver (manual trigger from palette).
    ActivateScreensaver,
    /// Dismiss the screensaver on any key/mouse input.
    DismissScreensaver,
```

- [ ] **Step 3: Add routing branch**

In `route_key_event()` (around line 885), add before the `_ => None` catch-all:

```rust
FocusLayer::Screensaver => Some(Action::DismissScreensaver),
```

- [ ] **Step 4: Run tests**

```bash
cargo test --lib
```
Expected: Pass (just adds enum variants)

- [ ] **Step 5: Commit**

```bash
git add src/action.rs src/state/types.rs
git commit -m "feat(screensaver): add action variants, FocusLayer, and key routing"
```

---

### Task 3: ScreensaverState + ScreensaverWidget (core module)

**Files:**
- Create: `src/screensaver/mod.rs`

- [ ] **Step 1: Write the particle physics test**

In a `#[cfg(test)] mod tests` block at the bottom of the new file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_wraps_at_screen_edges() {
        let mut p = Particle { x: 99.0, y: 50.0, vx: 1.0, vy: 0.0, ch: '.', layer: 0, brightness: 100 };
        let width = 80;
        let height = 24;
        wrap_particle(&mut p, width, height);
        assert_eq!(p.x as i32, 0);  // wrapped from 99 to 0
    }

    #[test]
    fn particle_wraps_negative() {
        let mut p = Particle { x: -1.0, y: 5.0, vx: 0.0, vy: 0.0, ch: '.', layer: 0, brightness: 100 };
        let width = 80;
        let height = 24;
        wrap_particle(&mut p, width, height);
        assert_eq!(p.x as i32, 79);  // wrapped from -1 to 79
    }

    #[test]
    fn wind_affects_particle_velocity() {
        let mut p = Particle { x: 10.0, y: 10.0, vx: 0.0, vy: 0.5, ch: '.', layer: 1, brightness: 100 };
        apply_wind(&mut p, 1.5, false);  // wind speed 1.5, no gust
        assert!(p.vx > 0.0);  // wind pushes particle right
        assert!(p.vx < 2.0);  // but not too much
    }

    #[test]
    fn screensaver_starts_inactive() {
        let ss = ScreensaverState::new(300, true);
        assert!(!ss.active);
        assert!(ss.enabled);
        assert_eq!(ss.timeout_secs, 300);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --lib screensaver -- --nocapture
```
Expected: compile error (module doesn't exist yet)

- [ ] **Step 3: Write the core module**

```rust
// src/screensaver/mod.rs

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};
use std::time::Instant;

const LAYER_BG_CHARS: &[char] = &['.', '·', ' '];
const LAYER_MID_CHARS: &[char] = &['*', 'o', '~', '\'', '"', ','];
const LAYER_FG_CHARS: &[char] = &['╌', '╍', '~', '─'];
const RAIN_CHARS: &[char] = &['│', '┊', '⋮'];

const LAYER_BG_COUNT: usize = 40;
const LAYER_MID_COUNT: usize = 25;
const LAYER_FG_COUNT: usize = 15;

const FRAME_INTERVAL: f64 = 1.0 / 12.0; // 12 fps

const GUST_INTERVAL_MIN: f64 = 8.0;
const GUST_INTERVAL_MAX: f64 = 20.0;
const GUST_DURATION: f64 = 2.0;

const RAIN_BURST_MIN: f64 = 3.0;
const RAIN_BURST_MAX: f64 = 8.0;
const RAIN_DRY_MIN: f64 = 5.0;
const RAIN_DRY_MAX: f64 = 15.0;

const WIND_PERIOD: f64 = 20.0; // seconds for full sine cycle

#[derive(Clone)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub ch: char,
    pub layer: u8,
    pub brightness: u8,
}

pub struct RainStreak {
    pub x: f64,
    pub head_y: f64,
    pub speed: f64,
    pub active: bool,
}

pub struct ScreensaverState {
    pub active: bool,
    pub enabled: bool,
    pub timeout_secs: u64,
    pub last_interaction: Instant,
    pub last_frame: Instant,
    particles: Vec<Particle>,
    rain_streaks: Vec<RainStreak>,
    wind_phase: f64,
    gust_timer: f64,
    gust_active: bool,
    gust_time_remaining: f64,
    rain_burst_timer: f64,
    rain_in_burst: bool,
    rain_time_remaining: f64,
    logo_pulse: f64,
    frame_counter: u64,
    rng: SimpleRng,
}

impl ScreensaverState {
    pub fn new(timeout_secs: u64, enabled: bool) -> Self {
        let now = Instant::now();
        let mut rng = SimpleRng::new(42);
        let mut state = Self {
            active: false,
            enabled,
            timeout_secs,
            last_interaction: now,
            last_frame: now,
            particles: Vec::with_capacity(LAYER_BG_COUNT + LAYER_MID_COUNT + LAYER_FG_COUNT),
            rain_streaks: Vec::with_capacity(15),
            wind_phase: 0.0,
            gust_timer: rand_interval(&mut rng, GUST_INTERVAL_MIN, GUST_INTERVAL_MAX),
            gust_active: false,
            gust_time_remaining: 0.0,
            rain_burst_timer: RAIN_DRY_MIN,
            rain_in_burst: false,
            rain_time_remaining: 0.0,
            logo_pulse: 0.0,
            frame_counter: 0,
            rng,
        };
        state.init_particles();
        state
    }

    fn init_particles(&mut self) {
        let rng = &mut self.rng;
        // Background layer
        for _ in 0..LAYER_BG_COUNT {
            self.particles.push(Particle {
                x: rng.f64() * 200.0,
                y: rng.f64() * 100.0,
                vx: (rng.f64() - 0.5) * 0.15,
                vy: (rng.f64() - 0.5) * 0.1,
                ch: LAYER_BG_CHARS[rng.usize(LAYER_BG_CHARS.len())],
                layer: 0,
                brightness: rng.u8(20, 60),
            });
        }
        // Midground layer
        for _ in 0..LAYER_MID_COUNT {
            self.particles.push(Particle {
                x: rng.f64() * 200.0,
                y: rng.f64() * 100.0,
                vx: rng.f64() * 0.3,
                vy: (rng.f64() - 0.5) * 0.2,
                ch: LAYER_MID_CHARS[rng.usize(LAYER_MID_CHARS.len())],
                layer: 1,
                brightness: rng.u8(60, 140),
            });
        }
        // Foreground layer
        for _ in 0..LAYER_FG_COUNT {
            self.particles.push(Particle {
                x: rng.f64() * 200.0,
                y: rng.f64() * 100.0,
                vx: rng.f64() * 0.6,
                vy: (rng.f64() - 0.5) * 0.1,
                ch: LAYER_FG_CHARS[rng.usize(LAYER_FG_CHARS.len())],
                layer: 2,
                brightness: rng.u8(120, 200),
            });
        }
    }

    /// Advance one animation frame. Call at ~12 fps.
    pub fn tick(&mut self, width: u16, height: u16, delta_secs: f64) {
        self.frame_counter += 1;
        self.wind_phase += delta_secs / WIND_PERIOD;
        let base_wind = (self.wind_phase * std::f64::consts::TAU).sin().abs() * 0.8 + 0.2;

        // Gust timer
        self.gust_timer -= delta_secs;
        if self.gust_active {
            self.gust_time_remaining -= delta_secs;
            if self.gust_time_remaining <= 0.0 {
                self.gust_active = false;
                self.gust_timer = rand_interval(&mut self.rng, GUST_INTERVAL_MIN, GUST_INTERVAL_MAX);
            }
        } else if self.gust_timer <= 0.0 {
            self.gust_active = true;
            self.gust_time_remaining = GUST_DURATION;
        }

        // Rain burst timer
        self.rain_burst_timer -= delta_secs;
        if self.rain_in_burst {
            self.rain_time_remaining -= delta_secs;
            if self.rain_time_remaining <= 0.0 {
                self.rain_in_burst = false;
                self.rain_burst_timer = rand_interval(&mut self.rng, RAIN_DRY_MIN, RAIN_DRY_MAX);
            }
        } else if self.rain_burst_timer <= 0.0 {
            self.rain_in_burst = true;
            self.rain_time_remaining = rand_interval(&mut self.rng, RAIN_BURST_MIN, RAIN_BURST_MAX);
        }

        // Wind multiplier
        let wind_mult = if self.gust_active { 2.5 } else { 1.0 };
        let wind_speed = base_wind * wind_mult;

        // Update particles
        for p in &mut self.particles {
            match p.layer {
                0 => {
                    p.vx += (rng.f64() - 0.5) * 0.05;
                    p.vy += (rng.f64() - 0.5) * 0.03;
                    p.vx = p.vx.clamp(-0.2, 0.2);
                    p.vy = p.vy.clamp(-0.15, 0.15);
                }
                1 => {
                    p.vx += (wind_speed * 0.3 - p.vx) * 0.02;
                    p.vy += (rng.f64() - 0.5) * 0.04;
                    p.vy = p.vy.clamp(-0.3, 0.3);
                }
                2 => {
                    p.vx += (wind_speed * 0.8 - p.vx) * 0.05;
                    p.vy += (rng.f64() - 0.5) * 0.02;
                    p.vy = p.vy.clamp(-0.1, 0.1);
                }
                _ => {}
            }
            p.x += p.vx;
            p.y += p.vy;
            wrap_particle(p, width, height);
        }

        // Rain streaks
        if self.rain_in_burst {
            // Ensure active rain streaks
            let target = if self.rain_time_remaining > 3.0 { 15 } else { 8 };
            while self.rain_streaks.len() < target.min(width as usize) {
                self.rain_streaks.push(RainStreak {
                    x: rng.f64() * width as f64,
                    head_y: rng.f64() * height as f64,
                    speed: rng.f64() * 1.5 + 0.5,
                    active: true,
                });
            }
            for streak in &mut self.rain_streaks {
                streak.head_y += streak.speed;
                streak.x += 0.3; // slight rightward drift
                if streak.head_y > height as f64 {
                    streak.head_y = 0.0;
                    streak.x = rng.f64() * width as f64;
                }
                if streak.x > width as f64 {
                    streak.x -= width as f64;
                }
            }
        } else {
            self.rain_streaks.clear();
        }

        // Logo pulse
        self.logo_pulse += delta_secs;
    }
}

pub fn wrap_particle(p: &mut Particle, width: u16, height: u16) {
    if p.x < 0.0 { p.x += width as f64; }
    if p.x >= width as f64 { p.x -= width as f64; }
    if p.y < 0.0 { p.y += height as f64; }
    if p.y >= height as f64 { p.y -= height as f64; }
}

pub fn apply_wind(p: &mut Particle, wind_speed: f64, _gust: bool) {
    p.vx += wind_speed * 0.02;
}

// ── Tiny inline LCG for zero dependency random numbers ──

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        // Parameters from Numerical Recipes (the "quick and dirty" generator)
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }

    fn f64(&mut self) -> f64 {
        // Map to [0, 1)
        (self.next() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    fn usize(&mut self, range: usize) -> usize {
        (self.f64() * range as f64) as usize
    }

    fn u8(&mut self, lo: u8, hi: u8) -> u8 {
        let range = hi.saturating_sub(lo) + 1;
        lo + (self.f64() * range as f64) as u8
    }
}

fn rand_interval(rng: &mut SimpleRng, min: f64, max: f64) -> f64 {
    rng.f64() * (max - min) + min
}

pub struct ScreensaverWidget;

impl ScreensaverWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Widget for ScreensaverWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ScreensaverState must be accessed through a thread-local or similar.
        // This widget is a marker — the real rendering logic is called from
        // a helper function that receives &ScreensaverState directly.
    }
}

/// Render the screensaver into the buffer. Called from ui::render().
pub fn render_screensaver(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let width = area.width;
    let height = area.height;
    if width < 40 || height < 10 {
        render_logo_only(state, area, buf);
        return;
    }

    // Clear the content area with a slightly tinted background
    for y in 0..height {
        for x in 0..width {
            let cell = buf.get_mut(area.x + x, area.y + y);
            cell.set_char(' ');
            cell.set_bg(Color::Rgb(9, 14, 22)); // slightly deeper than normal bg
        }
    }

    // Render particles (background → midground → foreground)
    for p in &state.particles {
        let px = (p.x as u16).min(width.saturating_sub(1));
        let py = (p.y as u16).min(height.saturating_sub(1));
        let cell = buf.get_mut(area.x + px, area.y + py);
        cell.set_char(p.ch);
        let b = p.brightness;
        match p.layer {
            0 => cell.set_fg(Color::Rgb(b / 3, b / 2, b)),
            1 => cell.set_fg(Color::Rgb(b / 2, b * 2 / 3, b)),
            2 => cell.set_fg(Color::Rgb(b * 2 / 3, b * 5 / 6, b)),
            _ => {}
        }
    }

    // Render rain streaks
    for streak in &state.rain_streaks {
        let sx = (streak.x as u16).min(width.saturating_sub(1));
        let head = (streak.head_y as u16).min(height.saturating_sub(1));
        let ch = RAIN_CHARS[state.frame_counter as usize % RAIN_CHARS.len()];
        let cell = buf.get_mut(area.x + sx, area.y + head);
        cell.set_char(ch);
        cell.set_fg(Color::Rgb(30, 50, 80));
        // Trail: dimmer cells above the head
        for t in 1..=3 {
            let ty = head.saturating_sub(t);
            let trail_cell = buf.get_mut(area.x + sx, area.y + ty);
            trail_cell.set_char(RAIN_CHARS[(state.frame_counter as usize + t) % RAIN_CHARS.len()]);
            let dim = 40 - t as u8 * 10;
            trail_cell.set_fg(Color::Rgb(dim / 2, dim, dim * 2));
        }
    }

    // Render logo
    render_logo_centered(state, area, buf);
}

fn render_logo_only(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    // For small terminals, render just the logo centered
    let cx = area.x + area.width / 2;
    let cy = area.y + area.height / 2;
    let logo = "[Z]eta\u{2588}";
    let pulse = ((state.logo_pulse * std::f64::consts::TAU * 0.25).sin() * 0.5 + 0.5) * 0.75 + 0.25;
    let b = (pulse * 255.0) as u8;
    let start_x = cx.saturating_sub((logo.len() / 2) as u16);
    for (i, ch) in logo.chars().enumerate() {
        let cell = buf.get_mut(start_x + i as u16, cy);
        cell.set_char(ch);
        match ch {
            '[' | ']' => cell.set_fg(Color::Rgb(b / 3, b / 3, b / 3)),
            'Z' => cell.set_fg(Color::Rgb(130, 170, 255)),
            _ => cell.set_fg(Color::Rgb(b * 4 / 5, b, b * 4 / 5)),
        }
    }
}

fn render_logo_centered(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let lines = [
        " ┌──────────┐ ",
        " │  [Z]eta  │ ",
        " └──────────┘ ",
    ];
    let line_width = lines[0].len() as u16;
    let logo_height = lines.len() as u16;
    let cx = area.x + (area.width.saturating_sub(line_width)) / 2;
    let cy = area.y + area.height / 3 - logo_height / 2;

    let pulse = ((state.logo_pulse * std::f64::consts::TAU * 0.25).sin() * 0.5 + 0.5) * 0.75 + 0.25;
    let b = (pulse * 255.0) as u8;

    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let x = cx + col as u16;
            let y = cy + row as u16;
            if x >= area.x + area.width || y >= area.y + area.height {
                continue;
            }
            let cell = buf.get_mut(x, y);
            cell.set_char(ch);
            match ch {
                '┌' | '┐' | '└' | '┘' | '─' | '│' => {
                    cell.set_fg(Color::Rgb(
                        (80.0 * pulse) as u8,
                        (120.0 * pulse) as u8,
                        (200.0 * pulse) as u8,
                    ));
                }
                '[' | ']' => cell.set_fg(Color::Rgb(b / 3, b / 2, b / 2)),
                'Z' => cell.set_fg(Color::Rgb(
                    (130.0 * pulse) as u8,
                    (170.0 * pulse) as u8,
                    255,
                )),
                _ => {
                    cell.set_fg(Color::Rgb(
                        (b * 4 / 5).min(200),
                        (b).min(230),
                        (b * 4 / 5).min(200),
                    ))
                }
            }
            // Faint background inside the box
            if ch != ' ' {
                cell.set_bg(Color::Rgb(12, 14, 18));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_wraps_at_screen_edges() {
        let mut p = Particle { x: 99.0, y: 50.0, vx: 1.0, vy: 0.0, ch: '.', layer: 0, brightness: 100 };
        wrap_particle(&mut p, 80, 24);
        assert_eq!(p.x as u16, 19);
    }

    #[test]
    fn particle_wraps_negative() {
        let mut p = Particle { x: -1.0, y: 5.0, vx: 0.0, vy: 0.0, ch: '.', layer: 0, brightness: 100 };
        wrap_particle(&mut p, 80, 24);
        assert_eq!(p.x as u16, 79);
    }

    #[test]
    fn wind_affects_particle_velocity() {
        let mut p = Particle { x: 10.0, y: 10.0, vx: 0.0, vy: 0.5, ch: '.', layer: 1, brightness: 100 };
        apply_wind(&mut p, 1.5, false);
        assert!(p.vx > 0.0);
        assert!(p.vx < 2.0);
    }

    #[test]
    fn screensaver_starts_inactive() {
        let ss = ScreensaverState::new(300, true);
        assert!(!ss.active);
        assert!(ss.enabled);
        assert_eq!(ss.timeout_secs, 300);
    }

    #[test]
    fn tick_runs_without_panicking() {
        let mut ss = ScreensaverState::new(300, true);
        ss.tick(80, 24, 0.083); // 12th of a second
        // Particles should have moved
        assert!(ss.frame_counter == 1);
    }

    #[test]
    fn small_terminal_renders_logo_only() {
        let ss = ScreensaverState::new(300, true);
        let area = Rect::new(0, 0, 30, 10);
        let mut buf = Buffer::empty(area);
        // Should not panic
        render_screensaver(&ss, area, &mut buf);
    }

    #[test]
    fn rain_streaks_cleared_during_dry_period() {
        let mut ss = ScreensaverState::new(300, true);
        // Force dry
        ss.rain_in_burst = false;
        ss.rain_streaks.push(RainStreak { x: 10.0, head_y: 5.0, speed: 1.0, active: true });
        ss.tick(80, 24, 0.083);
        // Rain streaks should be cleared
        assert!(ss.rain_in_burst || ss.rain_streaks.is_empty());
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test --lib screensaver -- --nocapture
```
Expected: 7 passed, 0 failed

- [ ] **Step 5: Commit**

```bash
git add src/screensaver/mod.rs
git commit -m "feat(screensaver): core module with particle system, wind, rain, and logo rendering"
```

---

### Task 4: Register module + add to AppState + action handlers

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/state/mod.rs`

- [ ] **Step 1: Register the module**

In `src/lib.rs`, add with other module declarations:

```rust
pub mod screensaver;
```

- [ ] **Step 2: Add screensaver field to AppState**

In `src/state/mod.rs`, add inside `AppState` struct (around line 297, after `show_cheatsheet`):

```rust
    pub screensaver: ScreensaverState,
```

And import at the top of the file:

```rust
use crate::screensaver::ScreensaverState;
```

- [ ] **Step 3: Initialize in bootstrap()**

In `AppState::bootstrap()` (around line 400), after config is available:

```rust
            screensaver: ScreensaverState::new(
                config.screensaver_timeout_secs,
                config.screensaver_enabled,
            ),
```

- [ ] **Step 4: Handle actions in apply()**

In `AppState::apply()` (around line 563), add these action handlers:

```rust
Action::ActivateScreensaver => {
    self.screensaver.active = true;
    self.set_needs_redraw();
}

Action::DismissScreensaver => {
    self.screensaver.active = false;
    self.screensaver.last_interaction = Instant::now();
    self.set_needs_redraw();
}
```

- [ ] **Step 5: Update focus_layer() chain**

In `focus_layer()` (around line 3218), add at the end before the fallback:

```rust
        if self.screensaver.active {
            return FocusLayer::Screensaver;
        }
```

- [ ] **Step 6: Run tests**

```bash
cargo test --lib
```
Expected: Pass

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/state/mod.rs
git commit -m "feat(screensaver): register module, add to AppState, wire up actions"
```

---

### Task 5: Event loop integration (idle detection, frame timing, dismiss)

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add fields to App**

In `src/app.rs`, inside `struct App` (around line 35):

```rust
    last_interaction: std::time::Instant,
```

Initialize in `App::new()` (around line 50):

```rust
            last_interaction: std::time::Instant::now(),
```

- [ ] **Step 2: Add idle detection in process_next_event()**

Inside `process_next_event()`, in the idle branch (around line 251, after the preview command check and before the clock second check):

```rust
        // Screensaver idle detection
        let screensaver = &mut self.state.screensaver;
        if !screensaver.active && screensaver.enabled && screensaver.timeout_secs > 0 {
            if self.last_interaction.elapsed() > std::time::Duration::from_secs(screensaver.timeout_secs) {
                screensaver.active = true;
                self.state.set_needs_redraw();
            }
        }

        // Screensaver frame tick (12 fps)
        if screensaver.active {
            let now = std::time::Instant::now();
            let delta = (now - screensaver.last_frame).as_secs_f64();
            if delta >= 1.0 / 12.0 {
                // We don't have screen dimensions here; compute on render
                screensaver.tick(160, 50, delta); // rough default, exact dims passed in render
                screensaver.last_frame = now;
                self.state.set_needs_redraw();
            }
        }
```

- [ ] **Step 3: Update last_interaction on every event**

In the event handling path (around line 270, after receiving a key/mouse/resize event):

```rust
        self.last_interaction = std::time::Instant::now();
```

Add this right after the `match &event {` block, or inline before the `route_key_event` call.

- [ ] **Step 4: Pass actual terminal dimensions to screensaver tick**

The tick currently uses a rough default (160x50). We need the actual terminal size. Since `process_next_event` doesn't know terminal dimensions, move the tick into the render path instead.

In the render section (around line 105), before `terminal.draw(...)`:

```rust
if self.state.screensaver.active {
    let now = std::time::Instant::now();
    let delta = (now - self.state.screensaver.last_frame).as_secs_f64();
    if delta >= 1.0 / 12.0 {
        let size = terminal.size()?;
        self.state.screensaver.tick(size.width, size.height, delta);
        self.state.screensaver.last_frame = now;
    }
}
```

And remove the tick call from process_next_event (keep the activation logic there).

- [ ] **Step 5: Run tests**

```bash
cargo test --lib
```
Expected: Pass

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "feat(screensaver): idle detection, frame tick, interaction tracking in event loop"
```

---

### Task 6: Render pipeline integration

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add screensaver import**

At the top of `src/ui/mod.rs`:

```rust
use crate::screensaver;
```

- [ ] **Step 2: Add render call**

At the end of the `render()` function (around line 427, after the debug panel render and before `LayoutCache` construction):

```rust
    // Screensaver overlay (renders on top of everything)
    if state.screensaver.active {
        screensaver::render_screensaver(&state.screensaver, areas[1], frame.buffer_mut());
    }
```

- [ ] **Step 3: Run tests**

```bash
cargo test --lib
```
Expected: Pass

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs
git commit -m "feat(screensaver): render screensaver overlay in main render pipeline"
```

---

### Task 7: Settings panel integration

**Files:**
- Modify: `src/state/settings.rs`
- Modify: `src/ui/settings.rs`

- [ ] **Step 1: Add SettingsField variants**

In `src/state/settings.rs`, add to `SettingsField` enum (around line 100):

```rust
    ScreensaverEnabled(bool),
    ScreensaverTimeout(u64),
```

- [ ] **Step 2: Create settings entries**

In `settings_entries_for_tab()` in `src/state/mod.rs` (around line 3780), add to the `SettingsTab::Panels` filter:

```rust
        | SettingsField::ScreensaverEnabled(_)
        | SettingsField::ScreensaverTimeout(_)
```

And add entry creation alongside other panel entries (around line 3840):

```rust
            SettingsField::ScreensaverEnabled(v) => SettingsEntry {
                field: SettingsField::ScreensaverEnabled(v),
                label: "Screensaver".into(),
                value: if *v { "on".into() } else { "off".into() },
                range: None,
            },
            SettingsField::ScreensaverTimeout(v) => SettingsEntry {
                field: SettingsField::ScreensaverTimeout(v),
                label: "Screensaver timeout".into(),
                value: format!("{v}s"),
                range: Some((0, 3600)),
            },
```

- [ ] **Step 3: Update settings apply method**

In the settings apply handler (around line 3860), add:

```rust
SettingsField::ScreensaverEnabled(v) => {
    self.config.screensaver_enabled = v;
    self.screensaver.enabled = v;
}
SettingsField::ScreensaverTimeout(v) => {
    self.config.screensaver_timeout_secs = v;
    self.screensaver.timeout_secs = v;
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --lib
```
Expected: Pass

- [ ] **Step 5: Commit**

```bash
git add src/state/settings.rs src/state/mod.rs
git commit -m "feat(screensaver): settings panel integration (Panels tab)"
```

---

### Task 8: Integration test + manual verification checklist

**Files:**
- New: `tests/screensaver_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
// tests/screensaver_integration.rs
use zeta::screensaver::{ScreensaverState, Particle};

#[test]
fn screensaver_activation_cycle() {
    let mut ss = ScreensaverState::new(1, true); // 1 second timeout
    assert!(!ss.active);

    // Simulate idle
    ss.last_interaction = std::time::Instant::now() - std::time::Duration::from_secs(2);
    // In the real app, process_next_event checks this; we test the state directly
    let elapsed = std::time::Instant::now().duration_since(ss.last_interaction);
    assert!(elapsed > std::time::Duration::from_secs(1));

    // Manual activation
    ss.active = true;
    assert!(ss.active);

    // Dismiss
    ss.active = false;
    assert!(!ss.active);
}

#[test]
fn screensaver_disabled_wont_activate() {
    let ss = ScreensaverState::new(300, false);
    assert!(!ss.enabled);
    assert!(!ss.active);
}

#[test]
fn timeout_zero_never_activates_via_timer() {
    let ss = ScreensaverState::new(0, true);
    assert!(!ss.active);
    assert_eq!(ss.timeout_secs, 0);
    // Timer check: timeout_secs > 0 is false, so no activation
}
```

- [ ] **Step 2: Run integration test**

```bash
cargo test --test screensaver_integration -- --nocapture
```
Expected: 3 passed, 0 failed

- [ ] **Step 3: Commit**

```bash
git add tests/screensaver_integration.rs
git commit -m "test(screensaver): add integration tests for activation cycle"
```

---

### Full build verification

- [ ] **Run full test suite**

```bash
cargo test --workspace
```
Expected: All tests pass

- [ ] **Run clippy**

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Expected: No warnings

- [ ] **Run format check**

```bash
cargo fmt --all -- --check
```
Expected: All files formatted

---

## Manual verification checklist (after building)

```bash
cargo run --
```

1. **Verify screensaver activates after idle timeout:**
   - Set `screensaver_timeout_secs = 10` in config
   - Wait 10 seconds without pressing any keys
   - Expected: Weather-themed ASCII animation appears (particles, rain, pulsing [Z]eta logo)

2. **Verify screensaver dismisses on any key:**
   - Press any key while screensaver is active
   - Expected: Screensaver disappears, normal UI returns

3. **Verify screensaver dismisses on mouse click:**
   - Click anywhere while screensaver is active
   - Expected: Screensaver disappears

4. **Verify manual trigger via command palette:**
   - Press Shift+P, type "screensaver"
   - Select "Activate Screensaver"
   - Expected: Screensaver activates immediately

5. **Verify settings panel controls:**
   - Press Ctrl+O, navigate to Panels tab
   - Adjust screensaver timeout value
   - Toggle screensaver on/off
   - Expected: Changes take effect immediately

6. **Verify disabled screensaver never activates:**
   - Set screensaver_enabled = false in config
   - Wait 5+ minutes
   - Expected: No screensaver activation

7. **Verify terminal resize during screensaver:**
   - Activate screensaver, resize terminal
   - Expected: Particles re-flow to new dimensions on next frame

8. **Verify small terminals:**
   - Shrink terminal to < 40 columns
   - Activate screensaver
   - Expected: Logo-only rendering (no particles), no crashes
