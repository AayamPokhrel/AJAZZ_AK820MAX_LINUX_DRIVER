use crate::spectrum::{AUDIO_BANDS, SpectrumAnalyzer};
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::{KEYS, TOTAL_LEDS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualizerStyle {
    Spectrum,
    Pulse,
    Wave,
    Passion,
    Matrix,
}

impl VisualizerStyle {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "spectrum" | "bars" | "equalizer" => Some(Self::Spectrum),
            "pulse" | "beat" => Some(Self::Pulse),
            "wave" | "ripple" => Some(Self::Wave),
            "passion" | "rock" | "fire" => Some(Self::Passion),
            "matrix" | "rain" => Some(Self::Matrix),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Spectrum => "spectrum",
            Self::Pulse => "pulse",
            Self::Wave => "wave",
            Self::Passion => "passion",
            Self::Matrix => "matrix",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorPalette {
    StockClassic,
    CyberpunkNeon,
    ThermalFire,
    OceanicIce,
}

impl ColorPalette {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "classic" | "stock" | "stockclassic" | "rainbow" => Some(Self::StockClassic),
            "cyberpunk" | "neon" | "cyberpunkneon" => Some(Self::CyberpunkNeon),
            "thermal" | "fire" | "thermalfire" | "heat" => Some(Self::ThermalFire),
            "oceanic" | "ice" | "oceanicice" | "water" => Some(Self::OceanicIce),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::StockClassic => "classic",
            Self::CyberpunkNeon => "cyberpunk",
            Self::ThermalFire => "thermal",
            Self::OceanicIce => "oceanic",
        }
    }

    /// Sample vertical gradient (0.0 = bottom, 1.0 = peak)
    pub fn sample(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::StockClassic => {
                if t < 0.5 {
                    let factor = t * 2.0;
                    Color::new((factor * 255.0) as u8, 255, 0)
                } else {
                    let factor = (t - 0.5) * 2.0;
                    Color::new(255, ((1.0 - factor) * 255.0) as u8, 0)
                }
            }
            Self::CyberpunkNeon => {
                if t < 0.5 {
                    let f = t * 2.0;
                    Color::new(
                        ((1.0 - f) * 255.0) as u8,
                        (f * 229.0) as u8,
                        (128.0 + f * 127.0) as u8,
                    )
                } else {
                    let f = (t - 0.5) * 2.0;
                    Color::new((f * 157.0) as u8, ((1.0 - f) * 229.0) as u8, 255)
                }
            }
            Self::ThermalFire => {
                if t < 0.4 {
                    let f = t / 0.4;
                    Color::new((120.0 + f * 135.0) as u8, 0, 0)
                } else if t < 0.8 {
                    let f = (t - 0.4) / 0.4;
                    Color::new(255, (f * 180.0) as u8, 0)
                } else {
                    let f = (t - 0.8) / 0.2;
                    Color::new(255, (180.0 + f * 75.0) as u8, (f * 200.0) as u8)
                }
            }
            Self::OceanicIce => {
                if t < 0.5 {
                    let f = t * 2.0;
                    Color::new(0, (40.0 + f * 160.0) as u8, (100.0 + f * 155.0) as u8)
                } else {
                    let f = (t - 0.5) * 2.0;
                    Color::new((f * 180.0) as u8, (200.0 + f * 55.0) as u8, 255)
                }
            }
        }
    }

    pub fn peak_color(&self) -> Color {
        match self {
            Self::StockClassic => Color::new(255, 255, 255),
            Self::CyberpunkNeon => Color::new(255, 255, 255),
            Self::ThermalFire => Color::new(255, 255, 200),
            Self::OceanicIce => Color::new(230, 255, 255),
        }
    }

    pub fn band_color(&self, band: usize, height_t: f32) -> Color {
        match self {
            Self::StockClassic => {
                // 16-band gradient hues across keyboard
                let hue = (band as f32 / AUDIO_BANDS as f32 * 0.85 + 0.55) % 1.0;
                let brightness = 0.75 + 0.25 * height_t;
                Color::from_hsv(hue, 1.0, brightness)
            }
            _ => self.sample(height_t),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneDirection {
    Horizontal,
    Vertical,
    Radial,
    Diagonal,
}

impl ZoneDirection {
    pub fn name(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Radial => "radial",
            Self::Diagonal => "diagonal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColorMode {
    Palette(ColorPalette),
    RandomPerKeyBeat,
    RandomPerKeyCycle,
    RandomPerKeyStatic,
    RandomZoneGradient { direction: ZoneDirection },
    CustomSingle(Color),
    CustomGradient { base: Color, peak: Color },
}

impl ColorMode {
    pub fn from_name(name: &str) -> Option<Self> {
        let clean = name.to_lowercase().replace(['-', '_', ' '], "");
        match clean.as_str() {
            "classic" | "stock" | "stockclassic" | "rainbow" => {
                Some(Self::Palette(ColorPalette::StockClassic))
            }
            "cyberpunk" | "neon" | "cyberpunkneon" => {
                Some(Self::Palette(ColorPalette::CyberpunkNeon))
            }
            "thermal" | "fire" | "thermalfire" | "heat" => {
                Some(Self::Palette(ColorPalette::ThermalFire))
            }
            "oceanic" | "ice" | "oceanicice" | "water" => {
                Some(Self::Palette(ColorPalette::OceanicIce))
            }
            "randomperkeybeat" | "perkeybeat" | "randombeat" => Some(Self::RandomPerKeyBeat),
            "randomperkeycycle" | "perkeycycle" | "randomcycle" | "flowing" => {
                Some(Self::RandomPerKeyCycle)
            }
            "randomperkeystatic" | "perkeystatic" | "mosaic" => Some(Self::RandomPerKeyStatic),
            "randomzonegradient" | "zonegradient" | "randomzone" | "horizontalzone" => {
                Some(Self::RandomZoneGradient {
                    direction: ZoneDirection::Horizontal,
                })
            }
            "verticalzone" => Some(Self::RandomZoneGradient {
                direction: ZoneDirection::Vertical,
            }),
            "radialzone" => Some(Self::RandomZoneGradient {
                direction: ZoneDirection::Radial,
            }),
            "diagonalzone" => Some(Self::RandomZoneGradient {
                direction: ZoneDirection::Diagonal,
            }),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Palette(p) => p.name(),
            Self::RandomPerKeyBeat => "random-beat",
            Self::RandomPerKeyCycle => "random-cycle",
            Self::RandomPerKeyStatic => "random-static",
            Self::RandomZoneGradient { direction } => direction.name(),
            Self::CustomSingle(_) => "custom-single",
            Self::CustomGradient { .. } => "custom-gradient",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualizerConfig {
    pub style: VisualizerStyle,
    pub color_mode: ColorMode,
    pub sensitivity: f32,
    pub noise_gate: f32,
    pub decay_speed: f32,
    pub peak_hold: bool,
    pub fps: u32,
    pub morph_speed: f32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            style: VisualizerStyle::Spectrum,
            color_mode: ColorMode::Palette(ColorPalette::StockClassic),
            sensitivity: 1.2,
            noise_gate: 0.04,
            decay_speed: 2.8,
            peak_hold: true,
            fps: 60,
            morph_speed: 1.5,
        }
    }
}

// Pseudo-random helper to avoid external rand dependency
fn pseudo_rand(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    x = x ^ (x >> 16);
    (x % 10000) as f32 / 10000.0
}

fn vibrant_hsv(hue: f32) -> Color {
    Color::from_hsv(hue % 1.0, 0.95, 1.0)
}

pub struct MusicRenderer {
    pub config: VisualizerConfig,
    bass_history: Vec<f32>,
    history_idx: usize,
    per_key_beat_colors: [Color; TOTAL_LEDS],
    per_key_was_active: [bool; TOTAL_LEDS],
    per_key_hues: [f32; TOTAL_LEDS],
    per_key_speeds: [f32; TOTAL_LEDS],
    static_mosaic: [Color; TOTAL_LEDS],
    zone_stops: [Color; 4],
    zone_target_stops: [Color; 4],
    rng_state: u32,
    total_time: f32,
}

impl MusicRenderer {
    pub fn new(style: VisualizerStyle, custom_color: Option<Color>) -> Self {
        let color_mode = if let Some(c) = custom_color {
            ColorMode::CustomGradient {
                base: c.scale(0.3),
                peak: c,
            }
        } else {
            ColorMode::Palette(ColorPalette::StockClassic)
        };

        let mut config = VisualizerConfig::default();
        config.style = style;
        config.color_mode = color_mode;

        let mut renderer = Self {
            config,
            bass_history: vec![0.0; 32],
            history_idx: 0,
            per_key_beat_colors: [Color::BLACK; TOTAL_LEDS],
            per_key_was_active: [false; TOTAL_LEDS],
            per_key_hues: [0.0; TOTAL_LEDS],
            per_key_speeds: [0.0; TOTAL_LEDS],
            static_mosaic: [Color::BLACK; TOTAL_LEDS],
            zone_stops: [Color::BLACK; 4],
            zone_target_stops: [Color::BLACK; 4],
            rng_state: 1337,
            total_time: 0.0,
        };

        renderer.init_random_modes();
        renderer
    }

    pub fn with_config(config: VisualizerConfig) -> Self {
        let mut renderer = Self {
            config,
            bass_history: vec![0.0; 32],
            history_idx: 0,
            per_key_beat_colors: [Color::BLACK; TOTAL_LEDS],
            per_key_was_active: [false; TOTAL_LEDS],
            per_key_hues: [0.0; TOTAL_LEDS],
            per_key_speeds: [0.0; TOTAL_LEDS],
            static_mosaic: [Color::BLACK; TOTAL_LEDS],
            zone_stops: [Color::BLACK; 4],
            zone_target_stops: [Color::BLACK; 4],
            rng_state: 1337,
            total_time: 0.0,
        };
        renderer.init_random_modes();
        renderer
    }

    fn next_rand(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng_state % 10000) as f32 / 10000.0
    }

    fn init_random_modes(&mut self) {
        for i in 0..TOTAL_LEDS {
            let h = pseudo_rand(i as u32 * 41 + 17);
            self.per_key_hues[i] = h;
            self.per_key_speeds[i] = 0.2 + 0.6 * pseudo_rand(i as u32 * 73 + 31);
            self.per_key_beat_colors[i] = vibrant_hsv(h);
            self.static_mosaic[i] = vibrant_hsv(pseudo_rand(i as u32 * 97 + 5));
        }

        self.zone_stops = [
            Color::new(255, 0, 128),  // Neon pink
            Color::new(140, 0, 255),  // Electric purple
            Color::new(0, 230, 255),  // Vivid cyan
            Color::new(255, 170, 0),  // Gold amber
        ];
        self.zone_target_stops = self.zone_stops;
    }

    /// Re-roll static mosaic colors for all keys
    pub fn reroll_mosaic(&mut self) {
        for i in 0..TOTAL_LEDS {
            let h = self.next_rand();
            self.static_mosaic[i] = vibrant_hsv(h);
        }
    }

    /// Shuffle multi-stop zone gradient colors immediately
    pub fn shuffle_zone_gradient(&mut self) {
        let base_hue = self.next_rand();
        for i in 0..4 {
            let h = (base_hue + (i as f32) * 0.28 + self.next_rand() * 0.08) % 1.0;
            self.zone_stops[i] = vibrant_hsv(h);
            let target_h = (h + 0.25 + self.next_rand() * 0.1) % 1.0;
            self.zone_target_stops[i] = vibrant_hsv(target_h);
        }
    }

    pub fn set_palette(&mut self, palette: ColorPalette) {
        self.config.color_mode = ColorMode::Palette(palette);
    }

    pub fn set_style(&mut self, style: VisualizerStyle) {
        self.config.style = style;
    }

    pub fn style(&self) -> VisualizerStyle {
        self.config.style
    }

    pub fn color_mode(&self) -> &ColorMode {
        &self.config.color_mode
    }

    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.config.color_mode = mode;
    }

    /// Smooth color interpolation across 4 multi-stop gradient zones
    fn sample_zone(&self, pos: f32) -> Color {
        let pos = pos.clamp(0.0, 1.0);
        let scaled = pos * 3.0;
        let idx = (scaled.floor() as usize).min(2);
        let f = scaled - idx as f32;
        let c1 = self.zone_stops[idx];
        let c2 = self.zone_stops[idx + 1];
        Color::new(
            (c1.r as f32 + (c2.r as f32 - c1.r as f32) * f) as u8,
            (c1.g as f32 + (c2.g as f32 - c1.g as f32) * f) as u8,
            (c1.b as f32 + (c2.b as f32 - c1.b as f32) * f) as u8,
        )
    }

    /// Decoupled color evaluator: maps key geometric & visualizer intensity to RGB color
    fn evaluate_key_color(
        &mut self,
        hw_idx: usize,
        stage_ratio: f32,
        intensity: f32,
        is_peak: bool,
        key_x: f32,
        key_y: f32,
        band: usize,
    ) -> Color {
        if intensity <= 0.01 && !is_peak {
            return Color::new(2, 4, 8); // Subtle ambient dark backlight
        }

        match &self.config.color_mode {
            ColorMode::Palette(p) => {
                if is_peak {
                    p.peak_color()
                } else if *p == ColorPalette::StockClassic {
                    p.band_color(band, stage_ratio).scale(intensity)
                } else {
                    p.sample(stage_ratio).scale(intensity)
                }
            }

            ColorMode::RandomPerKeyBeat => {
                if is_peak {
                    Color::new(255, 255, 255)
                } else {
                    let col = self.per_key_beat_colors[hw_idx.min(TOTAL_LEDS - 1)];
                    col.scale(intensity)
                }
            }

            ColorMode::RandomPerKeyCycle => {
                if is_peak {
                    Color::new(255, 255, 255)
                } else {
                    let hue = (self.per_key_hues[hw_idx.min(TOTAL_LEDS - 1)]
                        + self.total_time * self.per_key_speeds[hw_idx.min(TOTAL_LEDS - 1)])
                        % 1.0;
                    Color::from_hsv(hue, 0.95, intensity)
                }
            }

            ColorMode::RandomPerKeyStatic => {
                if is_peak {
                    Color::new(255, 255, 255)
                } else {
                    self.static_mosaic[hw_idx.min(TOTAL_LEDS - 1)].scale(intensity)
                }
            }

            ColorMode::RandomZoneGradient { direction } => {
                let pos = match direction {
                    ZoneDirection::Horizontal => ((key_x - 30.0) / 520.0).clamp(0.0, 1.0),
                    ZoneDirection::Vertical => (1.0 - (key_y - 40.0) / 180.0).clamp(0.0, 1.0),
                    ZoneDirection::Radial => {
                        let dx = (key_x - 295.0) / 265.0;
                        let dy = (key_y - 130.0) / 90.0;
                        (dx * dx + dy * dy).sqrt().clamp(0.0, 1.0)
                    }
                    ZoneDirection::Diagonal => {
                        let nx = ((key_x - 30.0) / 520.0).clamp(0.0, 1.0);
                        let ny = (1.0 - (key_y - 40.0) / 180.0).clamp(0.0, 1.0);
                        ((nx + ny) * 0.5).clamp(0.0, 1.0)
                    }
                };

                if is_peak {
                    Color::new(255, 255, 255)
                } else {
                    self.sample_zone(pos).scale(intensity)
                }
            }

            ColorMode::CustomSingle(c) => {
                if is_peak {
                    Color::new(255, 255, 255)
                } else {
                    c.scale(intensity)
                }
            }

            ColorMode::CustomGradient { base, peak } => {
                if is_peak {
                    *peak
                } else {
                    let r = (base.r as f32 + (peak.r as f32 - base.r as f32) * stage_ratio) as u8;
                    let g = (base.g as f32 + (peak.g as f32 - base.g as f32) * stage_ratio) as u8;
                    let b = (base.b as f32 + (peak.b as f32 - base.b as f32) * stage_ratio) as u8;
                    Color::new(r, g, b).scale(intensity)
                }
            }
        }
    }

    /// Render a full 108-LED frame based on decoupled (Style, ColorMode) computation
    pub fn render(&mut self, analyzer: &SpectrumAnalyzer, dt: f32) -> [Color; TOTAL_LEDS] {
        self.total_time += dt;
        let mut frame = [Color::BLACK; TOTAL_LEDS];
        let levels = analyzer.levels();
        let peaks = analyzer.peaks();
        let beat = analyzer.beat_detected;

        // Circular bass history for wave style
        self.bass_history[self.history_idx] = analyzer.bass;
        self.history_idx = (self.history_idx + 1) % self.bass_history.len();

        // 1. Beat-triggered per-key color reroll
        if beat {
            let base_h = self.next_rand();
            for i in 0..TOTAL_LEDS {
                if self.next_rand() < 0.65 {
                    let h = (base_h + self.next_rand() * 0.6) % 1.0;
                    self.per_key_beat_colors[i] = vibrant_hsv(h);
                }
            }
        }

        // 2. Zone Gradient continuous morphing & beat rotation
        let morph_rate = (self.config.morph_speed * dt).clamp(0.01, 0.4);
        for i in 0..4 {
            let cur = self.zone_stops[i];
            let tgt = self.zone_target_stops[i];
            self.zone_stops[i] = Color::new(
                (cur.r as f32 + (tgt.r as f32 - cur.r as f32) * morph_rate) as u8,
                (cur.g as f32 + (tgt.g as f32 - cur.g as f32) * morph_rate) as u8,
                (cur.b as f32 + (tgt.b as f32 - cur.b as f32) * morph_rate) as u8,
            );
        }

        if beat {
            // Strong beat: roll next targets
            let shift = self.next_rand();
            for i in 0..4 {
                let h = (shift + (i as f32) * 0.25) % 1.0;
                self.zone_target_stops[i] = vibrant_hsv(h);
            }
        }

        // Map the 16 frequency bands to 10 keyboard column bands
        let map_band = |col: usize| -> usize {
            ((col as f32 / 9.0) * (AUDIO_BANDS as f32 - 1.0)).round() as usize
        };

        // 3. Compute (intensity, is_peak) per key based on active visualizer style
        match self.config.style {
            VisualizerStyle::Spectrum => {
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = map_band(key.audio_band);
                    let level = levels[band.min(AUDIO_BANDS - 1)];
                    let peak = peaks[band.min(AUDIO_BANDS - 1)];

                    let stage_ratio = (key.stage as f32) / 5.0;
                    let peak_ratio = (peak * 5.0).round() / 5.0;

                    let (intensity, is_peak) = if stage_ratio <= level * 1.05 {
                        let factor = (stage_ratio * 0.3 + 0.70).clamp(0.0, 1.0);
                        (factor, false)
                    } else if (stage_ratio - peak_ratio).abs() < 0.12 && peak > 0.08 {
                        (1.0, true)
                    } else {
                        (0.0, false)
                    };

                    // Detect transition from unlit to lit for RandomPerKeyBeat
                    let is_active = intensity > 0.05;
                    if is_active && !self.per_key_was_active[key.hw_idx] {
                        let h = self.next_rand();
                        self.per_key_beat_colors[key.hw_idx] = vibrant_hsv(h);
                    }
                    self.per_key_was_active[key.hw_idx] = is_active;

                    frame[key.hw_idx] = self.evaluate_key_color(
                        key.hw_idx,
                        stage_ratio,
                        intensity,
                        is_peak,
                        key.rect.center_x(),
                        key.rect.center_y(),
                        band,
                    );
                }
            }

            VisualizerStyle::Pulse => {
                let energy = (0.05 + 0.95 * analyzer.bass.powf(1.4)).clamp(0.0, 1.0);
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let cx = key.rect.center_x() - 295.0;
                    let cy = key.rect.center_y() - 130.0;
                    let dist = (cx * cx + cy * cy).sqrt() / 250.0;
                    let radius = energy * 1.25;

                    let (intensity, is_peak) = if dist <= radius {
                        let t_grad = 1.0 - (dist / radius).clamp(0.0, 1.0);
                        let is_edge = (dist - radius).abs() < 0.12;
                        (t_grad, is_edge && beat)
                    } else {
                        (0.0, false)
                    };

                    let is_active = intensity > 0.05;
                    if is_active && !self.per_key_was_active[key.hw_idx] {
                        let h = self.next_rand();
                        self.per_key_beat_colors[key.hw_idx] = vibrant_hsv(h);
                    }
                    self.per_key_was_active[key.hw_idx] = is_active;

                    frame[key.hw_idx] = self.evaluate_key_color(
                        key.hw_idx,
                        1.0 - dist.min(1.0),
                        intensity,
                        is_peak,
                        key.rect.center_x(),
                        key.rect.center_y(),
                        0,
                    );
                }
            }

            VisualizerStyle::Wave => {
                let count = self.bass_history.len();
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let norm_x = ((key.rect.center_x() - 30.0) / 520.0).clamp(0.0, 1.0);
                    let hist_offset = (norm_x * (count as f32 - 1.0)) as usize;
                    let idx = (self.history_idx + count - 1 - hist_offset) % count;
                    let val = self.bass_history[idx];

                    let norm_y = (key.rect.center_y() - 40.0) / 180.0;
                    let height = (1.0 - norm_y).clamp(0.0, 1.0);

                    let (intensity, is_peak) = if height <= val * 1.15 {
                        (val.clamp(0.0, 1.0), (height - val).abs() < 0.10)
                    } else {
                        (0.0, false)
                    };

                    let is_active = intensity > 0.05;
                    if is_active && !self.per_key_was_active[key.hw_idx] {
                        let h = self.next_rand();
                        self.per_key_beat_colors[key.hw_idx] = vibrant_hsv(h);
                    }
                    self.per_key_was_active[key.hw_idx] = is_active;

                    frame[key.hw_idx] = self.evaluate_key_color(
                        key.hw_idx,
                        height,
                        intensity,
                        is_peak,
                        key.rect.center_x(),
                        key.rect.center_y(),
                        (norm_x * 15.0) as usize,
                    );
                }
            }

            VisualizerStyle::Passion => {
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = map_band(key.audio_band);
                    let level = levels[band.min(AUDIO_BANDS - 1)];
                    let norm_y = (key.rect.center_y() - 40.0) / 180.0;
                    let wave = (self.total_time * 6.0 + key.col as f32 * 0.8).sin() * 0.15;
                    let flame = (level * 1.35 - norm_y * 0.35 + wave).clamp(0.0, 1.0);

                    let (intensity, is_peak) = if flame > 0.12 {
                        (flame, flame > 0.88)
                    } else {
                        (0.0, false)
                    };

                    let is_active = intensity > 0.05;
                    if is_active && !self.per_key_was_active[key.hw_idx] {
                        let h = self.next_rand();
                        self.per_key_beat_colors[key.hw_idx] = vibrant_hsv(h);
                    }
                    self.per_key_was_active[key.hw_idx] = is_active;

                    frame[key.hw_idx] = self.evaluate_key_color(
                        key.hw_idx,
                        flame,
                        intensity,
                        is_peak,
                        key.rect.center_x(),
                        key.rect.center_y(),
                        band,
                    );
                }
            }

            VisualizerStyle::Matrix => {
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = map_band(key.audio_band);
                    let level = levels[band.min(AUDIO_BANDS - 1)];
                    let stage_ratio = (key.stage as f32) / 5.0;

                    let drop_phase = (self.total_time * 7.5 + (band as f32) * 1.6).fract();
                    let drop_dist = (stage_ratio - (1.0 - drop_phase)).abs();

                    let (intensity, is_peak) = if drop_dist < 0.24 && level > 0.08 {
                        let factor = (1.0 - drop_dist / 0.24) * level;
                        (factor, drop_dist < 0.08)
                    } else {
                        (0.0, false)
                    };

                    let is_active = intensity > 0.05;
                    if is_active && !self.per_key_was_active[key.hw_idx] {
                        let h = self.next_rand();
                        self.per_key_beat_colors[key.hw_idx] = vibrant_hsv(h);
                    }
                    self.per_key_was_active[key.hw_idx] = is_active;

                    frame[key.hw_idx] = self.evaluate_key_color(
                        key.hw_idx,
                        stage_ratio,
                        intensity,
                        is_peak,
                        key.rect.center_x(),
                        key.rect.center_y(),
                        band,
                    );
                }
            }
        }

        frame
    }
}
