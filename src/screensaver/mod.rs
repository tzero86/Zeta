use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use std::fmt;
use std::time::Instant;

const LAYER_BG_CHARS: &[char] = &['.', '·', ' '];
const LAYER_MID_CHARS: &[char] = &['*', 'o', '~', '\'', '"', ','];
const LAYER_FG_CHARS: &[char] = &['╌', '╍', '~', '─'];
const RAIN_CHARS: &[char] = &['│', '┊', '⋮'];

const LAYER_BG_COUNT: usize = 40;
const LAYER_MID_COUNT: usize = 25;
const LAYER_FG_COUNT: usize = 15;

const GUST_INTERVAL_MIN: f64 = 8.0;
const GUST_INTERVAL_MAX: f64 = 20.0;
const GUST_DURATION: f64 = 2.0;

const RAIN_BURST_MIN: f64 = 3.0;
const RAIN_BURST_MAX: f64 = 8.0;
const RAIN_DRY_MIN: f64 = 5.0;
const RAIN_DRY_MAX: f64 = 15.0;

const WIND_PERIOD: f64 = 20.0;

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

impl fmt::Debug for ScreensaverState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScreensaverState")
            .field("active", &self.active)
            .field("enabled", &self.enabled)
            .field("timeout_secs", &self.timeout_secs)
            .field("last_interaction", &self.last_interaction)
            .field("frame_counter", &self.frame_counter)
            .finish_non_exhaustive()
    }
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

    pub fn tick(&mut self, width: u16, height: u16, delta_secs: f64) {
        self.frame_counter += 1;
        self.wind_phase += delta_secs / WIND_PERIOD;
        let base_wind = (self.wind_phase * std::f64::consts::TAU).sin().abs() * 0.8 + 0.2;

        self.gust_timer -= delta_secs;
        if self.gust_active {
            self.gust_time_remaining -= delta_secs;
            if self.gust_time_remaining <= 0.0 {
                self.gust_active = false;
                self.gust_timer =
                    rand_interval(&mut self.rng, GUST_INTERVAL_MIN, GUST_INTERVAL_MAX);
            }
        } else if self.gust_timer <= 0.0 {
            self.gust_active = true;
            self.gust_time_remaining = GUST_DURATION;
        }

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

        let wind_mult = if self.gust_active { 2.5 } else { 1.0 };
        let wind_speed = base_wind * wind_mult;

        for p in &mut self.particles {
            match p.layer {
                0 => {
                    p.vx += (self.rng.f64() - 0.5) * 0.05;
                    p.vy += (self.rng.f64() - 0.5) * 0.03;
                    p.vx = p.vx.clamp(-0.2, 0.2);
                    p.vy = p.vy.clamp(-0.15, 0.15);
                }
                1 => {
                    p.vx += (wind_speed * 0.3 - p.vx) * 0.02;
                    p.vy += (self.rng.f64() - 0.5) * 0.04;
                    p.vy = p.vy.clamp(-0.3, 0.3);
                }
                2 => {
                    p.vx += (wind_speed * 0.8 - p.vx) * 0.05;
                    p.vy += (self.rng.f64() - 0.5) * 0.02;
                    p.vy = p.vy.clamp(-0.1, 0.1);
                }
                _ => {}
            }
            p.x += p.vx;
            p.y += p.vy;
            wrap_particle(p, width, height);
        }

        if self.rain_in_burst {
            let target = if self.rain_time_remaining > 3.0 {
                15
            } else {
                8
            };
            while self.rain_streaks.len() < target.min(width as usize) {
                self.rain_streaks.push(RainStreak {
                    x: self.rng.f64() * width as f64,
                    head_y: self.rng.f64() * height as f64,
                    speed: self.rng.f64() * 1.5 + 0.5,
                    active: true,
                });
            }
            for streak in &mut self.rain_streaks {
                streak.head_y += streak.speed;
                streak.x += 0.3;
                if streak.head_y > height as f64 {
                    streak.head_y = 0.0;
                    streak.x = self.rng.f64() * width as f64;
                }
                if streak.x > width as f64 {
                    streak.x -= width as f64;
                }
            }
        } else {
            self.rain_streaks.clear();
        }

        self.logo_pulse += delta_secs;
    }
}

pub fn wrap_particle(p: &mut Particle, width: u16, height: u16) {
    if p.x < 0.0 {
        p.x += width as f64;
    }
    if p.x >= width as f64 {
        p.x -= width as f64;
    }
    if p.y < 0.0 {
        p.y += height as f64;
    }
    if p.y >= height as f64 {
        p.y -= height as f64;
    }
}

// ── Tiny inline LCG for zero dependency random numbers ──

struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    fn usize(&mut self, range: usize) -> usize {
        (self.f64() * range as f64) as usize
    }

    fn u8(&mut self, lo: u8, hi: u8) -> u8 {
        let range = hi.saturating_sub(lo).saturating_add(1) as u16;
        let offset = (self.f64() * range as f64) as u16;
        (lo as u16 + offset).min(255) as u8
    }
}

fn rand_interval(rng: &mut SimpleRng, min: f64, max: f64) -> f64 {
    rng.f64() * (max - min) + min
}

pub fn render_screensaver(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let width = area.width;
    let height = area.height;
    if width < 40 || height < 10 {
        render_logo_only(state, area, buf);
        return;
    }

    for y in 0..height {
        for x in 0..width {
            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(9, 14, 22));
            }
        }
    }

    // Render particles background layer first, then midground, then foreground
    for p in &state.particles {
        let px = (p.x as u16).min(width.saturating_sub(1));
        let py = (p.y as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            cell.set_char(p.ch);
            let b = p.brightness;
            if p.layer == 0 {
                cell.set_fg(Color::Rgb(b / 3, b / 2, b));
            } else if p.layer == 1 {
                cell.set_fg(Color::Rgb(b / 2, b * 2 / 3, b));
            } else if p.layer == 2 {
                cell.set_fg(Color::Rgb(b * 2 / 3, b * 5 / 6, b));
            }
        }
    }

    // Render rain streaks
    for streak in &state.rain_streaks {
        let sx = (streak.x as u16).min(width.saturating_sub(1));
        let head = (streak.head_y as u16).min(height.saturating_sub(1));
        let ch = RAIN_CHARS[state.frame_counter as usize % RAIN_CHARS.len()];
        if let Some(cell) = buf.cell_mut((area.x + sx, area.y + head)) {
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(30, 50, 80));
        }
        for t in 1..=3 {
            let ty = head.saturating_sub(t);
            if let Some(trail_cell) = buf.cell_mut((area.x + sx, area.y + ty)) {
                trail_cell.set_char(
                    RAIN_CHARS[(state.frame_counter as usize + t as usize) % RAIN_CHARS.len()],
                );
                let dim = 40 - t as u8 * 10;
                trail_cell.set_fg(Color::Rgb(dim / 2, dim, dim * 2));
            }
        }
    }

    render_logo_centered(state, area, buf);
}

fn render_logo_only(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let cx = area.x + area.width / 2;
    let cy = area.y + area.height / 2;
    let logo = "[Z]eta\u{2588}";
    let pulse = ((state.logo_pulse * std::f64::consts::TAU * 0.25).sin() * 0.5 + 0.5) * 0.75 + 0.25;
    let b = (pulse * 255.0) as u8;
    let start_x = cx.saturating_sub((logo.len() / 2) as u16);
    for (i, ch) in logo.chars().enumerate() {
        let Some(cell) = buf.cell_mut((start_x + i as u16, cy)) else {
            continue;
        };
        cell.set_char(ch);
        match ch {
            '[' | ']' => {
                cell.set_fg(Color::Rgb(b / 3, b / 3, b / 3));
            }
            'Z' => {
                cell.set_fg(Color::Rgb(130, 170, 255));
            }
            _ => {
                cell.set_fg(Color::Rgb(
                    (b as u16 * 4 / 5) as u8,
                    b,
                    (b as u16 * 4 / 5) as u8,
                ));
            }
        }
    }
}

fn render_logo_centered(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let lines = [" ┌──────────┐ ", " │  [Z]eta  │ ", " └──────────┘ "];
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
            let Some(cell) = buf.cell_mut((x, y)) else {
                continue;
            };
            cell.set_char(ch);
            match ch {
                '┌' | '┐' | '└' | '┘' | '─' | '│' => {
                    cell.set_fg(Color::Rgb(
                        (80.0 * pulse) as u8,
                        (120.0 * pulse) as u8,
                        (200.0 * pulse) as u8,
                    ));
                }
                '[' | ']' => {
                    cell.set_fg(Color::Rgb(b / 3, b / 2, b / 2));
                }
                'Z' => {
                    cell.set_fg(Color::Rgb(
                        (130.0 * pulse) as u8,
                        (170.0 * pulse) as u8,
                        255,
                    ));
                }
                _ => {
                    cell.set_fg(Color::Rgb(
                        ((b as u16 * 4 / 5) as u8).min(200),
                        (b).min(230),
                        ((b as u16 * 4 / 5) as u8).min(200),
                    ));
                }
            }
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
        let mut p = Particle {
            x: 99.0,
            y: 50.0,
            vx: 1.0,
            vy: 0.0,
            ch: '.',
            layer: 0,
            brightness: 100,
        };
        wrap_particle(&mut p, 80, 24);
        assert_eq!(p.x as u16, 19);
    }

    #[test]
    fn particle_wraps_negative() {
        let mut p = Particle {
            x: -1.0,
            y: 5.0,
            vx: 0.0,
            vy: 0.0,
            ch: '.',
            layer: 0,
            brightness: 100,
        };
        wrap_particle(&mut p, 80, 24);
        assert_eq!(p.x as u16, 79);
    }

    #[test]
    fn screensaver_starts_inactive() {
        let ss = ScreensaverState::new(300, true);
        assert!(!ss.active);
        assert!(ss.enabled);
        assert_eq!(ss.timeout_secs, 300);
    }

    #[test]
    fn wind_affects_midground_particles() {
        let mut ss = ScreensaverState::new(300, true);
        for p in &mut ss.particles {
            if p.layer == 1 {
                p.vx = 0.0;
            }
        }
        // Multiple ticks simulate ~2.5 seconds of real time to accumulate wind
        for _ in 0..30 {
            ss.tick(80, 24, 0.083);
        }
        let got_wind = ss.particles.iter().any(|p| p.layer == 1 && p.vx > 0.001);
        assert!(
            got_wind,
            "midground particles should gain vx from wind after multiple ticks"
        );
    }

    #[test]
    fn small_terminal_renders_logo_only() {
        let ss = ScreensaverState::new(300, true);
        let area = Rect::new(0, 0, 30, 10);
        let mut buf = Buffer::empty(area);
        render_screensaver(&ss, area, &mut buf);
        // Logo "[Z]eta█" (9 bytes, 7 chars) centered at cx=15
        // start_x = 15 - (9/2) = 15 - 4 = 11
        // Position 11=[, 12=Z, 13=], 14=e, 15=t, 16=a, 17=█
        let cell = buf.cell((12, 5)).unwrap();
        assert_eq!(cell.symbol(), "Z");
        let cell = buf.cell((11, 5)).unwrap();
        assert_eq!(cell.symbol(), "[");
    }

    #[test]
    fn rain_streaks_cleared_during_dry_period() {
        let mut ss = ScreensaverState::new(300, true);
        ss.rain_in_burst = false;
        ss.rain_streaks.push(RainStreak {
            x: 10.0,
            head_y: 5.0,
            speed: 1.0,
            active: true,
        });
        ss.tick(80, 24, 0.083);
        assert!(ss.rain_in_burst || ss.rain_streaks.is_empty());
    }
}
