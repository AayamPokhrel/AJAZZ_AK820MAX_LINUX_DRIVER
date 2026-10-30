use crate::color::Color;
use crate::layout::{KEYS, TOTAL_LEDS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypingMode {
    RandomGradientSplash,
    ChaosRoulette,
    ShockwaveRipple,
    ThermalHeatmap,
    MatrixCodeRain,
    CrossfireLaser,
    StarlightSparks,
    CosmicAurora,
}

impl TypingMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::RandomGradientSplash => "Random Gradient Splash",
            Self::ChaosRoulette => "Chaos FX Roulette",
            Self::ShockwaveRipple => "Neon Shockwave",
            Self::ThermalHeatmap => "Thermal Fire Heatmap",
            Self::MatrixCodeRain => "Matrix Code Rain",
            Self::CrossfireLaser => "Crossfire Lasers",
            Self::StarlightSparks => "Starlight Supernova",
            Self::CosmicAurora => "Cosmic Aurora Surge",
        }
    }
}

#[derive(Debug, Clone)]
struct Ripple {
    x: f32,
    y: f32,
    radius: f32,
    max_radius: f32,
    speed: f32,
    color: Color,
    lifetime: f32,
    age: f32,
}

#[derive(Debug, Clone)]
struct LaserBeam {
    x: f32,
    y: f32,
    color: Color,
    age: f32,
    lifetime: f32,
}

#[derive(Debug, Clone)]
struct SparkParticle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    color: Color,
    age: f32,
    lifetime: f32,
}

#[derive(Debug, Clone)]
struct MatrixDrop {
    #[allow(dead_code)]
    x: f32,
    col: usize,
    y: f32,
    speed: f32,
    color: Color,
    age: f32,
    lifetime: f32,
}

pub struct TypingEngine {
    pub mode: TypingMode,
    rng_state: u64,
    // Splashes & Ripples
    ripples: Vec<Ripple>,
    // Thermal simulation
    temperatures: [f32; TOTAL_LEDS],
    // Lasers
    lasers: Vec<LaserBeam>,
    // Particles
    sparks: Vec<SparkParticle>,
    // Matrix drops
    matrix_drops: Vec<MatrixDrop>,
    // Gradient Splash State
    current_gradient_stops: [Color; 4],
    target_gradient_stops: [Color; 4],
    gradient_transition: f32,
    // Key press flash intensity
    key_flashes: [f32; TOTAL_LEDS],
    // Cosmic aurora phase
    aurora_phase: f32,
    aurora_surge: f32,
    // Key press burst counter for roulette
    burst_count: u32,
    total_keystrokes: u64,
}

impl Default for TypingEngine {
    fn default() -> Self {
        Self::new(TypingMode::RandomGradientSplash)
    }
}

impl TypingEngine {
    pub fn new(mode: TypingMode) -> Self {
        let mut engine = Self {
            mode,
            rng_state: 0x9E3779B97F4A7C15,
            ripples: Vec::with_capacity(32),
            temperatures: [0.0; TOTAL_LEDS],
            lasers: Vec::with_capacity(32),
            sparks: Vec::with_capacity(128),
            matrix_drops: Vec::with_capacity(32),
            current_gradient_stops: [
                Color::new(0, 242, 254),
                Color::new(79, 172, 254),
                Color::new(168, 85, 247),
                Color::new(244, 63, 94),
            ],
            target_gradient_stops: [
                Color::new(0, 242, 254),
                Color::new(79, 172, 254),
                Color::new(168, 85, 247),
                Color::new(244, 63, 94),
            ],
            gradient_transition: 1.0,
            key_flashes: [0.0; TOTAL_LEDS],
            aurora_phase: 0.0,
            aurora_surge: 0.0,
            burst_count: 0,
            total_keystrokes: 0,
        };
        engine.roll_new_gradient();
        engine
    }

    fn lcg_rand(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let val = (self.rng_state >> 32) as u32;
        val as f32 / 4294967296.0
    }

    fn random_neon_color(&mut self) -> Color {
        let h = self.lcg_rand();
        Color::from_hsv(h, 0.95, 1.0)
    }

    pub fn roll_new_gradient(&mut self) {
        let base_hue = self.lcg_rand();
        self.current_gradient_stops = self.target_gradient_stops;
        self.target_gradient_stops = [
            Color::from_hsv(base_hue, 0.95, 1.0),
            Color::from_hsv((base_hue + 0.15).fract(), 0.90, 0.95),
            Color::from_hsv((base_hue + 0.35).fract(), 0.95, 1.0),
            Color::from_hsv((base_hue + 0.55).fract(), 0.90, 0.95),
        ];
        self.gradient_transition = 0.0;
    }

    pub fn set_mode(&mut self, mode: TypingMode) {
        self.mode = mode;
        self.ripples.clear();
        self.lasers.clear();
        self.sparks.clear();
        self.matrix_drops.clear();
    }

    pub fn on_key_down(&mut self, hw_idx: usize, _row: usize, col: usize, x: f32, y: f32) {
        self.total_keystrokes = self.total_keystrokes.wrapping_add(1);
        self.burst_count = self.burst_count.wrapping_add(1);

        if hw_idx < TOTAL_LEDS {
            self.key_flashes[hw_idx] = 1.0;
        }

        match self.mode {
            TypingMode::RandomGradientSplash => {
                // Every keypress rolls/advances gradient and triggers radiant shockwave
                if self.burst_count % 3 == 0 || self.gradient_transition >= 0.8 {
                    self.roll_new_gradient();
                }

                let col = self.random_neon_color();
                self.ripples.push(Ripple {
                    x,
                    y,
                    radius: 0.0,
                    max_radius: 550.0,
                    speed: 480.0,
                    color: col,
                    lifetime: 1.1,
                    age: 0.0,
                });
            }

            TypingMode::ChaosRoulette => {
                // Random effect on every keypress!
                let fx_type = (self.lcg_rand() * 5.0) as u32;
                let neon_color = self.random_neon_color();

                match fx_type {
                    0 => {
                        // Laser
                        self.lasers.push(LaserBeam { x, y, color: neon_color, age: 0.0, lifetime: 0.6 });
                    }
                    1 => {
                        // Shockwave
                        self.ripples.push(Ripple {
                            x,
                            y,
                            radius: 0.0,
                            max_radius: 400.0,
                            speed: 520.0,
                            color: neon_color,
                            lifetime: 0.8,
                            age: 0.0,
                        });
                    }
                    2 => {
                        // Thermal injection
                        if hw_idx < TOTAL_LEDS {
                            self.temperatures[hw_idx] = (self.temperatures[hw_idx] + 1.2).min(2.5);
                        }
                    }
                    3 => {
                        // Sparks explosion
                        for _ in 0..10 {
                            let angle = self.lcg_rand() * std::f32::consts::PI * 2.0;
                            let spd = 60.0 + self.lcg_rand() * 180.0;
                            let lifetime = 0.5 + self.lcg_rand() * 0.4;
                            self.sparks.push(SparkParticle {
                                x,
                                y,
                                vx: angle.cos() * spd,
                                vy: angle.sin() * spd,
                                color: neon_color,
                                age: 0.0,
                                lifetime,
                            });
                        }
                    }
                    _ => {
                        // Matrix drop
                        self.matrix_drops.push(MatrixDrop {
                            x,
                            col,
                            y: 0.0,
                            speed: 380.0,
                            color: neon_color,
                            age: 0.0,
                            lifetime: 0.9,
                        });
                    }
                }
            }

            TypingMode::ShockwaveRipple => {
                let col = self.random_neon_color();
                self.ripples.push(Ripple {
                    x,
                    y,
                    radius: 0.0,
                    max_radius: 520.0,
                    speed: 460.0,
                    color: col,
                    lifetime: 0.9,
                    age: 0.0,
                });
            }

            TypingMode::ThermalHeatmap => {
                if hw_idx < TOTAL_LEDS {
                    self.temperatures[hw_idx] = (self.temperatures[hw_idx] + 0.85).min(2.5);
                }
            }

            TypingMode::MatrixCodeRain => {
                let col_neon = Color::new(0, 255, 65);
                let speed = 320.0 + self.lcg_rand() * 120.0;
                self.matrix_drops.push(MatrixDrop {
                    x,
                    col,
                    y: 0.0,
                    speed,
                    color: col_neon,
                    age: 0.0,
                    lifetime: 0.8,
                });
            }

            TypingMode::CrossfireLaser => {
                let col = self.random_neon_color();
                self.lasers.push(LaserBeam { x, y, color: col, age: 0.0, lifetime: 0.55 });
            }

            TypingMode::StarlightSparks => {
                let col = self.random_neon_color();
                for _ in 0..10 {
                    let angle = self.lcg_rand() * std::f32::consts::PI * 2.0;
                    let spd = 70.0 + self.lcg_rand() * 160.0;
                    let lifetime = 0.6 + self.lcg_rand() * 0.4;
                    self.sparks.push(SparkParticle {
                        x,
                        y,
                        vx: angle.cos() * spd,
                        vy: angle.sin() * spd,
                        color: col,
                        age: 0.0,
                        lifetime,
                    });
                }
            }

            TypingMode::CosmicAurora => {
                self.aurora_surge = (self.aurora_surge + 0.4).min(1.5);
                let col = self.random_neon_color();
                self.ripples.push(Ripple {
                    x,
                    y,
                    radius: 0.0,
                    max_radius: 350.0,
                    speed: 380.0,
                    color: col,
                    lifetime: 0.7,
                    age: 0.0,
                });
            }
        }
    }

    pub fn update(&mut self, dt: f32) -> [Color; TOTAL_LEDS] {
        let mut frame = [Color::BLACK; TOTAL_LEDS];
        let dt_clamped = dt.clamp(0.001, 0.1);

        // Update key flashes
        for flash in self.key_flashes.iter_mut() {
            *flash = (*flash - dt_clamped * 3.5).max(0.0);
        }

        match self.mode {
            TypingMode::RandomGradientSplash => {
                self.gradient_transition = (self.gradient_transition + dt_clamped * 1.5).min(1.0);
                let t_interp = self.gradient_transition;

                // Base smooth spatial gradient
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let u = (key.rect.center_x() / 600.0).clamp(0.0, 1.0);
                        let c_curr = sample_4_gradient(&self.current_gradient_stops, u);
                        let c_next = sample_4_gradient(&self.target_gradient_stops, u);
                        let base_col = c_curr.lerp(c_next, t_interp).scale(0.35);
                        frame[key.hw_idx] = base_col;
                    }
                }

                // Apply active expanding ripples
                self.update_and_render_ripples(&mut frame, dt_clamped);
            }

            TypingMode::ChaosRoulette => {
                // Dark cyber base
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = Color::new(10, 12, 20);
                    }
                }

                self.update_and_render_ripples(&mut frame, dt_clamped);
                self.update_and_render_lasers(&mut frame, dt_clamped);
                self.update_and_render_sparks(&mut frame, dt_clamped);
                self.update_and_render_matrix(&mut frame, dt_clamped);
                self.update_and_render_thermal(&mut frame, dt_clamped);
            }

            TypingMode::ShockwaveRipple => {
                // Deep navy base
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = Color::new(4, 6, 14);
                    }
                }
                self.update_and_render_ripples(&mut frame, dt_clamped);
            }

            TypingMode::ThermalHeatmap => {
                self.update_and_render_thermal(&mut frame, dt_clamped);
            }

            TypingMode::MatrixCodeRain => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = Color::new(0, 14, 4);
                    }
                }
                self.update_and_render_matrix(&mut frame, dt_clamped);
            }

            TypingMode::CrossfireLaser => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = Color::new(8, 8, 16);
                    }
                }
                self.update_and_render_lasers(&mut frame, dt_clamped);
            }

            TypingMode::StarlightSparks => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = Color::new(6, 6, 16);
                    }
                }
                self.update_and_render_sparks(&mut frame, dt_clamped);
            }

            TypingMode::CosmicAurora => {
                self.aurora_phase += dt_clamped * (1.2 + self.aurora_surge * 2.0);
                self.aurora_surge = (self.aurora_surge - dt_clamped * 0.8).max(0.0);

                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let x_norm = key.rect.center_x() / 600.0;
                        let y_norm = key.rect.center_y() / 250.0;

                        let wave1 = (x_norm * 4.0 + self.aurora_phase).sin() * 0.5 + 0.5;
                        let wave2 = (y_norm * 3.0 - self.aurora_phase * 0.7).cos() * 0.5 + 0.5;
                        let wave = (wave1 * 0.6 + wave2 * 0.4).powf(1.4);

                        let hue = 0.5 + 0.3 * ((self.aurora_phase * 0.2 + x_norm * 0.4).sin() * 0.5 + 0.5);
                        let bri = (0.2 + wave * 0.8 * (1.0 + self.aurora_surge)).clamp(0.0, 1.0);
                        frame[key.hw_idx] = Color::from_hsv(hue, 0.9, bri);
                    }
                }
                self.update_and_render_ripples(&mut frame, dt_clamped);
            }
        }

        // Add incandescent key flash on pressed keys
        for key in KEYS {
            if key.hw_idx < TOTAL_LEDS {
                let flash = self.key_flashes[key.hw_idx];
                if flash > 0.01 {
                    let white_spark = Color::WHITE.scale(flash);
                    frame[key.hw_idx] = frame[key.hw_idx].add(white_spark);
                }
            }
        }

        frame
    }

    fn update_and_render_ripples(&mut self, frame: &mut [Color; TOTAL_LEDS], dt: f32) {
        for rip in self.ripples.iter_mut() {
            rip.age += dt;
            rip.radius += rip.speed * dt;
        }
        self.ripples.retain(|r| r.age < r.lifetime && r.radius < r.max_radius);

        for rip in &self.ripples {
            let progress = rip.age / rip.lifetime;
            let fade = (1.0 - progress).powf(1.6);
            if fade <= 0.01 {
                continue;
            }

            for key in KEYS {
                if key.hw_idx < TOTAL_LEDS {
                    let dx = key.rect.center_x() - rip.x;
                    let dy = key.rect.center_y() - rip.y;
                    let dist = (dx * dx + dy * dy).sqrt();

                    let dist_diff = (dist - rip.radius).abs();
                    let thickness = 32.0;
                    if dist_diff < thickness {
                        let intensity = (1.0 - dist_diff / thickness).powf(2.0) * fade;
                        let wave_col = rip.color.scale(intensity);
                        frame[key.hw_idx] = frame[key.hw_idx].add(wave_col);
                    }
                }
            }
        }
    }

    fn update_and_render_thermal(&mut self, frame: &mut [Color; TOTAL_LEDS], dt: f32) {
        // Physical heat dissipation
        let decay = (0.35f32).powf(dt);
        for temp in self.temperatures.iter_mut() {
            *temp *= decay;
        }

        for key in KEYS {
            if key.hw_idx < TOTAL_LEDS {
                let t = self.temperatures[key.hw_idx];
                let col = temperature_to_fire_color(t);
                frame[key.hw_idx] = frame[key.hw_idx].add(col);
            }
        }
    }

    fn update_and_render_lasers(&mut self, frame: &mut [Color; TOTAL_LEDS], dt: f32) {
        for laser in self.lasers.iter_mut() {
            laser.age += dt;
        }
        self.lasers.retain(|l| l.age < l.lifetime);

        for laser in &self.lasers {
            let progress = laser.age / laser.lifetime;
            let fade = (1.0 - progress).powf(1.8);
            if fade <= 0.01 {
                continue;
            }

            for key in KEYS {
                if key.hw_idx < TOTAL_LEDS {
                    let dx = (key.rect.center_x() - laser.x).abs();
                    let dy = (key.rect.center_y() - laser.y).abs();

                    // Horizontal laser beam
                    let int_h = if dy < 20.0 { (1.0 - dy / 20.0) * 0.9 * fade } else { 0.0 };
                    // Vertical laser beam
                    let int_v = if dx < 20.0 { (1.0 - dx / 20.0) * 0.9 * fade } else { 0.0 };

                    let total_int = (int_h + int_v).min(1.0);
                    if total_int > 0.01 {
                        let beam_col = laser.color.scale(total_int);
                        frame[key.hw_idx] = frame[key.hw_idx].add(beam_col);
                    }
                }
            }
        }
    }

    fn update_and_render_sparks(&mut self, frame: &mut [Color; TOTAL_LEDS], dt: f32) {
        for spark in self.sparks.iter_mut() {
            spark.age += dt;
            spark.x += spark.vx * dt;
            spark.y += spark.vy * dt;
            spark.vy += 60.0 * dt; // gravity
            spark.vx *= (0.5f32).powf(dt); // drag
        }
        self.sparks.retain(|s| s.age < s.lifetime);

        for spark in &self.sparks {
            let progress = spark.age / spark.lifetime;
            let fade = 1.0 - progress;
            if fade <= 0.01 {
                continue;
            }

            for key in KEYS {
                if key.hw_idx < TOTAL_LEDS {
                    let dx = key.rect.center_x() - spark.x;
                    let dy = key.rect.center_y() - spark.y;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq < 25.0 * 25.0 {
                        let int = (1.0 - dist_sq / (25.0 * 25.0)) * fade;
                        let c = spark.color.scale(int);
                        frame[key.hw_idx] = frame[key.hw_idx].add(c);
                    }
                }
            }
        }
    }

    fn update_and_render_matrix(&mut self, frame: &mut [Color; TOTAL_LEDS], dt: f32) {
        for drop in self.matrix_drops.iter_mut() {
            drop.age += dt;
            drop.y += drop.speed * dt;
        }
        self.matrix_drops.retain(|d| d.age < d.lifetime && d.y < 300.0);

        for drop in &self.matrix_drops {
            for key in KEYS {
                if key.hw_idx < TOTAL_LEDS && key.col == drop.col {
                    let dy = drop.y - key.rect.center_y();
                    if dy >= -10.0 && dy <= 60.0 {
                        // Head is bright white-green, tail is phosphorescent green
                        let int = 1.0 - (dy + 10.0) / 70.0;
                        let c = if dy < 10.0 {
                            Color::new(200, 255, 220).scale(int)
                        } else {
                            drop.color.scale(int * 0.85)
                        };
                        frame[key.hw_idx] = frame[key.hw_idx].add(c);
                    }
                }
            }
        }
    }
}

fn sample_4_gradient(stops: &[Color; 4], u: f32) -> Color {
    let u_clamped = u.clamp(0.0, 1.0);
    if u_clamped <= 0.333 {
        let f = u_clamped / 0.333;
        stops[0].lerp(stops[1], f)
    } else if u_clamped <= 0.666 {
        let f = (u_clamped - 0.333) / 0.333;
        stops[1].lerp(stops[2], f)
    } else {
        let f = (u_clamped - 0.666) / 0.334;
        stops[2].lerp(stops[3], f)
    }
}

fn temperature_to_fire_color(t: f32) -> Color {
    let t = t.clamp(0.0, 2.5);
    if t <= 0.05 {
        Color::BLACK
    } else if t <= 0.35 {
        // Black to dark crimson
        let f = (t - 0.05) / 0.30;
        Color::BLACK.lerp(Color::new(170, 0, 10), f)
    } else if t <= 0.75 {
        // Crimson to fiery blaze orange
        let f = (t - 0.35) / 0.40;
        Color::new(170, 0, 10).lerp(Color::new(255, 75, 0), f)
    } else if t <= 1.25 {
        // Orange to golden incandescent
        let f = (t - 0.75) / 0.50;
        Color::new(255, 75, 0).lerp(Color::new(255, 210, 30), f)
    } else {
        // Golden to blinding white-hot plasma
        let f = ((t - 1.25) / 1.25).min(1.0);
        Color::new(255, 210, 30).lerp(Color::new(255, 255, 255), f)
    }
}
