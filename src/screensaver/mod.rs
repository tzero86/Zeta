use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use std::fmt;
use std::time::Instant;

const LAYER_BG_CHARS: &[char] = &['.', '·'];
const LAYER_MID_CHARS: &[char] = &['*', 'o', '~', '\'', '"', ','];
const LAYER_FG_CHARS: &[char] = &['╌', '╍', '~', '─'];
const RAIN_CHARS: &[char] = &['│', '┊', '⋮'];
const SNOW_CHARS: &[char] = &['*', '.', '·', '•'];
const METEOR_CHARS: &[char] = &['─', '╌', '╍', '━'];
const SPARKLE_CHARS: &[char] = &['✧', '✦', '⋆', '✶', '·'];

const LAYER_BG_COUNT: usize = 50;
const LAYER_MID_COUNT: usize = 30;
const LAYER_FG_COUNT: usize = 18;
const ORBITER_COUNT: usize = 10;
const COMET_COUNT: usize = 4;
const SPARKLE_COUNT: usize = 4;

const GUST_INTERVAL_MIN: f64 = 8.0;
const GUST_INTERVAL_MAX: f64 = 20.0;
const GUST_DURATION: f64 = 2.5;

const RAIN_BURST_MIN: f64 = 3.0;
const RAIN_BURST_MAX: f64 = 8.0;
const RAIN_DRY_MIN: f64 = 8.0;
const RAIN_DRY_MAX: f64 = 20.0;

const SNOW_DURATION_MIN: f64 = 20.0;
const SNOW_DURATION_MAX: f64 = 60.0;
const SNOW_PAUSE_MIN: f64 = 30.0;
const SNOW_PAUSE_MAX: f64 = 90.0;

const WIND_PERIOD: f64 = 20.0;

const METEOR_INTERVAL_MIN: f64 = 5.0;
const METEOR_INTERVAL_MAX: f64 = 15.0;
const METEOR_LIFETIME: f64 = 0.6;

const LIGHTNING_INTERVAL_MIN: f64 = 10.0;
const LIGHTNING_INTERVAL_MAX: f64 = 30.0;

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

struct Meteor {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    trail: Vec<(f64, f64, u8)>,
    life: f64,
}

struct Orbiter {
    angle: f64,
    radius_x: f64,
    radius_y: f64,
    speed: f64,
    ch: char,
    brightness: u8,
}

struct Comet {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    ch: char,
    trail: Vec<(f64, f64, u8)>,
}

struct Sparkle {
    x: f64,
    y: f64,
    ch: char,
    phase: f64,
    speed: f64,
}

struct LightningBolt {
    life: f64,
    segments: Vec<(f64, f64)>,
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
    snow_timer: f64,
    snow_active: bool,
    snow_time_remaining: f64,
    logo_pulse: f64,
    frame_counter: u64,
    rng: SimpleRng,
    meteors: Vec<Meteor>,
    meteor_timer: f64,
    orbiters: Vec<Orbiter>,
    comets: Vec<Comet>,
    sparkles: Vec<Sparkle>,
    lightning_bolts: Vec<LightningBolt>,
    lightning_timer: f64,
    lightning_flash: u8,
    color_shift: f64,
    snow_particles: Vec<Particle>,
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
            snow_timer: SNOW_PAUSE_MIN,
            snow_active: false,
            snow_time_remaining: 0.0,
            logo_pulse: 0.0,
            frame_counter: 0,
            meteor_timer: rand_interval(&mut rng, METEOR_INTERVAL_MIN, METEOR_INTERVAL_MAX),
            lightning_timer: rand_interval(
                &mut rng,
                LIGHTNING_INTERVAL_MIN,
                LIGHTNING_INTERVAL_MAX,
            ),
            rng,
            meteors: Vec::with_capacity(5),
            orbiters: Vec::with_capacity(ORBITER_COUNT),
            comets: Vec::with_capacity(COMET_COUNT),
            sparkles: Vec::with_capacity(SPARKLE_COUNT),
            lightning_bolts: Vec::with_capacity(3),
            lightning_flash: 0,
            color_shift: 0.0,
            snow_particles: Vec::with_capacity(60),
        };
        state.init_particles();
        state.init_orbiters();
        state.init_comets(80, 24);
        state.init_sparkles();
        state
    }

    fn init_particles(&mut self) {
        let rng = &mut self.rng;
        for _ in 0..LAYER_BG_COUNT {
            self.particles.push(Particle {
                x: rng.f64() * 200.0,
                y: rng.f64() * 100.0,
                vx: (rng.f64() - 0.5) * 0.12,
                vy: (rng.f64() - 0.5) * 0.08,
                ch: LAYER_BG_CHARS[rng.usize(LAYER_BG_CHARS.len())],
                layer: 0,
                brightness: rng.u8(15, 45),
            });
        }
        for _ in 0..LAYER_MID_COUNT {
            self.particles.push(Particle {
                x: rng.f64() * 200.0,
                y: rng.f64() * 100.0,
                vx: rng.f64() * 0.25,
                vy: (rng.f64() - 0.5) * 0.18,
                ch: LAYER_MID_CHARS[rng.usize(LAYER_MID_CHARS.len())],
                layer: 1,
                brightness: rng.u8(50, 130),
            });
        }
        for _ in 0..LAYER_FG_COUNT {
            self.particles.push(Particle {
                x: rng.f64() * 200.0,
                y: rng.f64() * 100.0,
                vx: rng.f64() * 0.5,
                vy: (rng.f64() - 0.5) * 0.08,
                ch: LAYER_FG_CHARS[rng.usize(LAYER_FG_CHARS.len())],
                layer: 2,
                brightness: rng.u8(110, 190),
            });
        }
    }

    fn init_orbiters(&mut self) {
        for _ in 0..ORBITER_COUNT {
            let angle = self.rng.f64() * std::f64::consts::TAU;
            let rx = 8.0 + self.rng.f64() * 6.0;
            let ry = 3.0 + self.rng.f64() * 2.0;
            self.orbiters.push(Orbiter {
                angle,
                radius_x: rx,
                radius_y: ry,
                speed: 0.3 + self.rng.f64() * 0.4,
                ch: SPARKLE_CHARS[self.rng.usize(SPARKLE_CHARS.len())],
                brightness: self.rng.u8(120, 220),
            });
        }
    }

    fn init_comets(&mut self, width: u16, height: u16) {
        for _ in 0..COMET_COUNT {
            let c = self.random_comet(width, height);
            self.comets.push(c);
        }
    }

    fn random_comet(&mut self, width: u16, height: u16) -> Comet {
        let x = self.rng.f64() * width as f64;
        let y = self.rng.f64() * height as f64 * 0.5;
        let angle = -0.5 + self.rng.f64() * 0.6;
        let speed = 2.0 + self.rng.f64() * 3.0;
        Comet {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin().abs() * speed * 0.5,
            ch: METEOR_CHARS[self.rng.usize(METEOR_CHARS.len())],
            trail: Vec::with_capacity(6),
        }
    }

    fn init_sparkles(&mut self) {
        for _ in 0..SPARKLE_COUNT {
            self.sparkles.push(Sparkle {
                x: self.rng.f64() * 200.0,
                y: self.rng.f64() * 100.0,
                ch: SPARKLE_CHARS[self.rng.usize(SPARKLE_CHARS.len())],
                phase: self.rng.f64() * std::f64::consts::TAU,
                speed: 0.5 + self.rng.f64() * 0.8,
            });
        }
    }

    pub fn tick(&mut self, width: u16, height: u16, delta_secs: f64) {
        self.frame_counter += 1;
        self.color_shift += delta_secs * 0.3;
        self.wind_phase += delta_secs / WIND_PERIOD;
        let base_wind = (self.wind_phase * std::f64::consts::TAU).sin().abs() * 0.8 + 0.2;

        // Gusts
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

        // Rain / snow toggle
        self.snow_timer -= delta_secs;
        if self.snow_active {
            self.snow_time_remaining -= delta_secs;
            if self.snow_time_remaining <= 0.0 {
                self.snow_active = false;
                self.snow_timer = rand_interval(&mut self.rng, SNOW_PAUSE_MIN, SNOW_PAUSE_MAX);
            }
        } else if self.snow_timer <= 0.0 {
            self.snow_active = true;
            self.snow_time_remaining =
                rand_interval(&mut self.rng, SNOW_DURATION_MIN, SNOW_DURATION_MAX);
        }

        // Rain
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

        // Regular particles
        for p in &mut self.particles {
            match p.layer {
                0 => {
                    p.vx += (self.rng.f64() - 0.5) * 0.04;
                    p.vy += (self.rng.f64() - 0.5) * 0.025;
                    p.vx = p.vx.clamp(-0.18, 0.18);
                    p.vy = p.vy.clamp(-0.12, 0.12);
                }
                1 => {
                    p.vx += (wind_speed * 0.25 - p.vx) * 0.018;
                    p.vy += (self.rng.f64() - 0.5) * 0.035;
                    p.vy = p.vy.clamp(-0.25, 0.25);
                }
                2 => {
                    p.vx += (wind_speed * 0.7 - p.vx) * 0.04;
                    p.vy += (self.rng.f64() - 0.5) * 0.015;
                    p.vy = p.vy.clamp(-0.08, 0.08);
                }
                _ => {}
            }
            p.x += p.vx;
            p.y += p.vy;
            wrap_particle(p, width, height);
        }

        // Snow particles
        if self.snow_active {
            while self.snow_particles.len() < 60 {
                self.snow_particles.push(Particle {
                    x: self.rng.f64() * width as f64,
                    y: 0.0,
                    vx: (self.rng.f64() - 0.5) * 0.3,
                    vy: 0.3 + self.rng.f64() * 0.5,
                    ch: SNOW_CHARS[self.rng.usize(SNOW_CHARS.len())],
                    layer: 0,
                    brightness: self.rng.u8(160, 240),
                });
            }
            for p in &mut self.snow_particles {
                p.x += p.vx + wind_speed * 0.15;
                p.y += p.vy;
                p.vx += (self.rng.f64() - 0.5) * 0.04;
                p.vx = p.vx.clamp(-0.5, 0.5);
                if p.y > height as f64 + 2.0 {
                    p.y = 0.0;
                    p.x = self.rng.f64() * width as f64;
                }
                if p.x > width as f64 {
                    p.x -= width as f64;
                }
                if p.x < 0.0 {
                    p.x += width as f64;
                }
            }
        } else {
            self.snow_particles.clear();
        }

        // Rain streaks
        if self.rain_in_burst && !self.snow_active {
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

        // Meteors
        self.meteor_timer -= delta_secs;
        if self.meteor_timer <= 0.0 {
            let count = 1 + (self.rng.usize(3));
            for _ in 0..count {
                let x = self.rng.f64() * width as f64;
                let y = self.rng.f64() * height as f64 * 0.3;
                let speed = 8.0 + self.rng.f64() * 6.0;
                let angle = self.rng.f64() * 0.4 - 0.2;
                self.meteors.push(Meteor {
                    x,
                    y,
                    vx: angle.cos() * speed,
                    vy: angle.sin().abs() * speed,
                    trail: Vec::with_capacity(12),
                    life: METEOR_LIFETIME,
                });
            }
            self.meteor_timer =
                rand_interval(&mut self.rng, METEOR_INTERVAL_MIN, METEOR_INTERVAL_MAX);
        }
        for m in &mut self.meteors {
            m.trail.push((m.x, m.y, 255));
            m.x += m.vx * delta_secs;
            m.y += m.vy * delta_secs;
            m.life -= delta_secs;
            for (_, _, br) in &mut m.trail {
                *br = br.saturating_sub(4);
            }
            m.trail.retain(|(_, _, br)| *br > 30);
            if m.trail.len() > 14 {
                m.trail.remove(0);
            }
        }
        self.meteors.retain(|m| m.life > 0.0);

        // Lightning
        self.lightning_timer -= delta_secs;
        if self.lightning_timer <= 0.0 {
            self.lightning_flash = 200;
            for _ in 0..(1 + self.rng.usize(2)) {
                let x1 = self.rng.f64() * width as f64 * 0.8 + width as f64 * 0.1;
                let x2 = x1 + (self.rng.f64() - 0.5) * 8.0;
                let mut segments = Vec::with_capacity(10);
                let steps = 6 + self.rng.usize(6);
                let mut cy = 1.0;
                for _ in 0..steps {
                    cy += 2.0 + self.rng.f64() * 2.0;
                    let cx = x2 + (self.rng.f64() - 0.5) * 5.0;
                    segments.push((cx, cy));
                }
                self.lightning_bolts.push(LightningBolt {
                    life: 0.35,
                    segments,
                });
            }
            self.lightning_timer = rand_interval(
                &mut self.rng,
                LIGHTNING_INTERVAL_MIN,
                LIGHTNING_INTERVAL_MAX,
            );
        }
        if self.lightning_flash > 0 {
            self.lightning_flash = self.lightning_flash.saturating_sub(40);
        }
        for b in &mut self.lightning_bolts {
            b.life -= delta_secs;
        }
        self.lightning_bolts.retain(|b| b.life > 0.0);

        // Orbiters
        for orb in &mut self.orbiters {
            orb.angle += orb.speed * delta_secs;
            if orb.angle > std::f64::consts::TAU {
                orb.angle -= std::f64::consts::TAU;
            }
            orb.brightness = if self.gust_active {
                orb.brightness.saturating_add(3).min(240)
            } else {
                orb.brightness.saturating_sub(1).max(100)
            };
        }

        // Comets
        let mut reset_indices = Vec::new();
        for (idx, c) in self.comets.iter_mut().enumerate() {
            c.trail.push((c.x, c.y, 180));
            c.x += c.vx * delta_secs;
            c.y += c.vy * delta_secs;
            for (_, _, br) in &mut c.trail {
                *br = br.saturating_sub(12);
            }
            c.trail.retain(|(_, _, br)| *br > 20);
            if c.trail.len() > 8 {
                c.trail.remove(0);
            }
            if c.x > width as f64 + 5.0 || c.x < -5.0 || c.y > height as f64 + 5.0 || c.y < -5.0 {
                reset_indices.push(idx);
            }
        }
        for idx in reset_indices {
            self.comets[idx] = self.random_comet(width, height);
        }

        // Sparkles
        for s in &mut self.sparkles {
            s.x += (s.phase.cos() * 0.15 + self.rng.f64() * 0.05 - 0.025) * delta_secs * 12.0;
            s.y += (s.phase.sin() * 0.1 + self.rng.f64() * 0.03 - 0.015) * delta_secs * 12.0;
            s.phase += s.speed * delta_secs;
            wrap_f64(&mut s.x, width as f64);
            wrap_f64(&mut s.y, height as f64);
        }

        self.logo_pulse += delta_secs;
    }
}

pub fn wrap_particle(p: &mut Particle, width: u16, height: u16) {
    wrap_f64(&mut p.x, width as f64);
    wrap_f64(&mut p.y, height as f64);
}

fn wrap_f64(v: &mut f64, limit: f64) {
    if *v < 0.0 {
        *v += limit;
    }
    if *v >= limit {
        *v -= limit;
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

// ── Rendering ──

pub fn render_screensaver(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let width = area.width;
    let height = area.height;
    if width < 40 || height < 10 {
        render_logo_only(state, area, buf);
        return;
    }

    let bg_shift = (state.color_shift.sin() * 0.5 + 0.5) * 15.0;

    for y in 0..height {
        for x in 0..width {
            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(
                    (8 + bg_shift as u8).min(30),
                    (12 + bg_shift as u8).min(30),
                    (20 + bg_shift as u8).min(40),
                ));
            }
        }
    }

    // Lightning flash overlay
    if state.lightning_flash > 0 {
        let f = state.lightning_flash;
        for y in 0..height {
            for x in 0..width {
                if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                    cell.set_bg(Color::Rgb(
                        (20 + (f as u16 * 2 / 3) as u8).min(200),
                        (20 + (f as u16 * 2 / 3) as u8).min(200),
                        30 + f,
                    ));
                }
            }
        }
    }

    // Snow particles
    for p in &state.snow_particles {
        let px = (p.x as u16).min(width.saturating_sub(1));
        let py = (p.y as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            cell.set_char(p.ch);
            cell.set_fg(Color::Rgb(200, 210, 230));
            cell.set_bg(Color::Reset);
        }
    }

    // Background particles
    for p in state.particles.iter().filter(|p| p.layer == 0) {
        let px = (p.x as u16).min(width.saturating_sub(1));
        let py = (p.y as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            cell.set_char(p.ch);
            let b = p.brightness;
            cell.set_fg(Color::Rgb(b / 4, b / 3, b / 2));
            cell.set_bg(Color::Reset);
        }
    }

    // Midground particles
    for p in state.particles.iter().filter(|p| p.layer == 1) {
        let px = (p.x as u16).min(width.saturating_sub(1));
        let py = (p.y as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            cell.set_char(p.ch);
            let b = p.brightness as u16;
            cell.set_fg(Color::Rgb((b / 2) as u8, (b * 2 / 3) as u8, b as u8));
            cell.set_bg(Color::Reset);
        }
    }

    for p in state.particles.iter().filter(|p| p.layer == 2) {
        let px = (p.x as u16).min(width.saturating_sub(1));
        let py = (p.y as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            cell.set_char(p.ch);
            let b = p.brightness as u16;
            cell.set_fg(Color::Rgb((b * 2 / 3) as u8, (b * 5 / 6) as u8, b as u8));
            cell.set_bg(Color::Reset);
        }
    }

    // Comets
    for c in &state.comets {
        for (i, (tx, ty, br)) in c.trail.iter().enumerate() {
            let px = (*tx as u16).min(width.saturating_sub(1));
            let py = (*ty as u16).min(height.saturating_sub(1));
            if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
                if i == c.trail.len().saturating_sub(1) {
                    cell.set_char(c.ch);
                } else {
                    cell.set_char('·');
                }
                let b = *br;
                cell.set_fg(Color::Rgb(b, b * 2 / 3, b / 4));
                cell.set_bg(Color::Reset);
            }
        }
    }

    // Meteors
    for m in &state.meteors {
        for (i, (mx, my, br)) in m.trail.iter().enumerate() {
            let px = (*mx as u16).min(width.saturating_sub(1));
            let py = (*my as u16).min(height.saturating_sub(1));
            if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
                cell.set_char(METEOR_CHARS[i % METEOR_CHARS.len()]);
                let b = *br;
                cell.set_fg(Color::Rgb(b, b, b * 2 / 3));
                cell.set_bg(Color::Reset);
            }
        }
    }

    // Rain streaks
    for streak in &state.rain_streaks {
        let sx = (streak.x as u16).min(width.saturating_sub(1));
        let head = (streak.head_y as u16).min(height.saturating_sub(1));
        let ch = RAIN_CHARS[state.frame_counter as usize % RAIN_CHARS.len()];
        if let Some(cell) = buf.cell_mut((area.x + sx, area.y + head)) {
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(50, 80, 130));
            cell.set_bg(Color::Reset);
        }
        for t in 1..=3 {
            let ty = head.saturating_sub(t);
            if let Some(cell) = buf.cell_mut((area.x + sx, area.y + ty)) {
                cell.set_char(
                    RAIN_CHARS[(state.frame_counter as usize + t as usize) % RAIN_CHARS.len()],
                );
                let dim = 50u8.saturating_sub(t as u8 * 12);
                cell.set_fg(Color::Rgb(dim / 3, dim / 2, dim));
                cell.set_bg(Color::Reset);
            }
        }
    }

    // Lightning bolts
    for bolt in &state.lightning_bolts {
        let brightness = (bolt.life / 0.35 * 200.0) as u8;
        for &(bx, by) in &bolt.segments {
            let px = (bx as u16).min(width.saturating_sub(1));
            let py = (by as u16).min(height.saturating_sub(1));
            if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
                cell.set_char('│');
                cell.set_fg(Color::Rgb(180, 200, brightness));
                cell.set_bg(Color::Rgb(brightness / 4, brightness / 3, brightness / 2));
            }
        }
    }

    // Sparkles (wanderers)
    for s in &state.sparkles {
        let px = (s.x as u16).min(width.saturating_sub(1));
        let py = (s.y as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            let b = ((s.phase.sin() * 0.5 + 0.5) * 180.0) as u8 + 60;
            cell.set_char(s.ch);
            cell.set_fg(Color::Rgb(130, b, 255));
            cell.set_bg(Color::Reset);
        }
    }

    // Orbiters around the logo (rendered as a halo ring)
    let logo_cx = area.x as f64 + area.width as f64 / 2.0;
    let logo_cy = area.y as f64 + area.height as f64 / 3.0;
    for orb in &state.orbiters {
        let ox = logo_cx + orb.angle.cos() * orb.radius_x;
        let oy = logo_cy + orb.angle.sin() * orb.radius_y;
        let px = (ox as u16).min(width.saturating_sub(1));
        let py = (oy as u16).min(height.saturating_sub(1));
        if let Some(cell) = buf.cell_mut((area.x + px, area.y + py)) {
            cell.set_char(orb.ch);
            let b = orb.brightness;
            cell.set_fg(Color::Rgb(100, b, 255));
            cell.set_bg(Color::Reset);
        }
    }

    render_logo_enhanced(state, area, buf);
    render_fade_text(state, area, buf);
}

fn render_fade_text(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let elapsed = (state.frame_counter as f64) / 12.0;
    let alpha = if elapsed < 8.0 {
        0
    } else {
        ((elapsed - 8.0).min(2.0) / 2.0 * 180.0) as u8
    };
    if alpha == 0 {
        return;
    }
    let text = "  Press any key to dismiss  ";
    let tx = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
    let ty = area.y + area.height.saturating_sub(3);
    for (i, ch) in text.chars().enumerate() {
        if let Some(cell) = buf.cell_mut((tx + i as u16, ty)) {
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(alpha / 3, alpha / 2, alpha));
            cell.set_bg(Color::Reset);
        }
    }
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
        if ch == '[' || ch == ']' {
            cell.set_fg(Color::Rgb(b / 3, b / 3, b / 3));
        } else if ch == 'Z' {
            cell.set_fg(Color::Rgb(130, 170, 255));
        } else {
            cell.set_fg(Color::Rgb(
                (b as u16 * 4 / 5) as u8,
                b,
                (b as u16 * 4 / 5) as u8,
            ));
        }
    }
}

fn render_logo_enhanced(state: &ScreensaverState, area: Rect, buf: &mut Buffer) {
    let lines = [
        "  ╭──────────────────╮  ",
        "  │  ▐▀▀▀▀▀▀▀▀▀▀▀▀▀▌  │  ",
        "  │  ▐   ▐██▌▐██▌  ▌  │  ",
        "  │  ▐    ██  ██   ▌  │  ",
        "  │  ▐   ▐██▌ ██   ▌  │  ",
        "  │  ▐    ██  ██   ▌  │  ",
        "  │  ▐   ▐██▌▐██▌  ▌  │  ",
        "  │  ▐▄▄▄▄▄▄▄▄▄▄▄▄▄▌  │  ",
        "  │      [Z]eta        │  ",
        "  ╰──────────────────╯  ",
    ];
    let line_width = lines[0].len() as u16;
    let logo_height = lines.len() as u16;
    let cx = area.x + (area.width.saturating_sub(line_width)) / 2;
    let cy = area.y + area.height / 3 - logo_height / 2;

    let pulse = ((state.logo_pulse * std::f64::consts::TAU * 0.25).sin() * 0.5 + 0.5) * 0.75 + 0.25;
    let b8 = (pulse * 255.0) as u8;

    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let x = cx + col as u16;
            let y = cy + row as u16;
            let Some(cell) = buf.cell_mut((x, y)) else {
                continue;
            };
            match ch {
                '╭' | '╮' | '╰' | '╯' | '─' | '│' | '╞' | '╡' => {
                    cell.set_char(ch);
                    let r = (80.0 * pulse) as u8;
                    let g = (140.0 * pulse) as u8;
                    let b = (220.0 * pulse) as u8;
                    cell.set_fg(Color::Rgb(r, g, b));
                    cell.set_bg(Color::Rgb(12, 14, 18));
                }
                '▐' | '▌' | '▀' | '▄' | '█' => {
                    cell.set_char(' ');
                    let r = (40.0 * pulse) as u8;
                    let g = (80.0 * pulse) as u8;
                    let b = (180.0 * pulse).min(200.0) as u8;
                    cell.set_bg(Color::Rgb(r, g, b));
                }
                '[' | ']' => {
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(b8 / 3, b8 / 2, b8 / 2));
                    cell.set_bg(Color::Rgb(12, 14, 18));
                }
                'Z' => {
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(
                        (130.0 * pulse) as u8,
                        (170.0 * pulse) as u8,
                        (220.0 * pulse).min(255.0) as u8,
                    ));
                    cell.set_bg(Color::Rgb(12, 14, 18));
                }
                ' ' => {
                    cell.set_bg(Color::Rgb(12, 14, 18));
                }
                _ => {
                    cell.set_char(ch);
                    let b = b8 as u16;
                    cell.set_fg(Color::Rgb(
                        (b * 4 / 5).min(200) as u8,
                        b8.min(230),
                        (b * 4 / 5).min(200) as u8,
                    ));
                    cell.set_bg(Color::Rgb(12, 14, 18));
                }
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

    #[test]
    fn orbiters_initialized() {
        let ss = ScreensaverState::new(300, true);
        assert_eq!(ss.orbiters.len(), ORBITER_COUNT);
    }

    #[test]
    fn comets_initialized() {
        let ss = ScreensaverState::new(300, true);
        assert_eq!(ss.comets.len(), COMET_COUNT);
    }

    #[test]
    fn tick_runs_without_panicking() {
        let mut ss = ScreensaverState::new(300, true);
        ss.tick(80, 24, 0.083);
        assert!(ss.frame_counter == 1);
    }
}
