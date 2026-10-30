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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorPalette {
    StockClassic,
    CyberpunkNeon,
    ThermalFire,
    OceanicIce,
    CustomGradient { base: Color, peak: Color },
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
            Self::CustomGradient { .. } => "custom",
        }
    }

    /// Sample color along a normalized height gradient (0.0 = bottom, 1.0 = peak)
    pub fn sample(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::StockClassic => {
                // Classic vendor green-yellow-red vertical gradient
                if t < 0.5 {
                    let factor = t * 2.0;
                    Color::new(
                        (factor * 255.0) as u8,
                        255,
                        0,
                    )
                } else {
                    let factor = (t - 0.5) * 2.0;
                    Color::new(
                        255,
                        ((1.0 - factor) * 255.0) as u8,
                        0,
                    )
                }
            }
            Self::CyberpunkNeon => {
                // Magenta (#FF0080) -> Electric Cyan (#00E5FF) -> Violet (#9D00FF)
                if t < 0.5 {
                    let f = t * 2.0;
                    Color::new(
                        ((1.0 - f) * 255.0) as u8,
                        (f * 229.0) as u8,
                        (128.0 + f * 127.0) as u8,
                    )
                } else {
                    let f = (t - 0.5) * 2.0;
                    Color::new(
                        (f * 157.0) as u8,
                        ((1.0 - f) * 229.0) as u8,
                        255,
                    )
                }
            }
            Self::ThermalFire => {
                // Deep Crimson Red -> Fiery Orange -> Vivid Yellow/White
                if t < 0.4 {
                    let f = t / 0.4;
                    Color::new(
                        (120.0 + f * 135.0) as u8,
                        0,
                        0,
                    )
                } else if t < 0.8 {
                    let f = (t - 0.4) / 0.4;
                    Color::new(
                        255,
                        (f * 180.0) as u8,
                        0,
                    )
                } else {
                    let f = (t - 0.8) / 0.2;
                    Color::new(
                        255,
                        (180.0 + f * 75.0) as u8,
                        (f * 200.0) as u8,
                    )
                }
            }
            Self::OceanicIce => {
                // Deep Navy -> Vivid Cyan -> Ice Mint White
                if t < 0.5 {
                    let f = t * 2.0;
                    Color::new(
                        0,
                        (40.0 + f * 160.0) as u8,
                        (100.0 + f * 155.0) as u8,
                    )
                } else {
                    let f = (t - 0.5) * 2.0;
                    Color::new(
                        (f * 180.0) as u8,
                        (200.0 + f * 55.0) as u8,
                        255,
                    )
                }
            }
            Self::CustomGradient { base, peak } => {
                Color::new(
                    (base.r as f32 + (peak.r as f32 - base.r as f32) * t) as u8,
                    (base.g as f32 + (peak.g as f32 - base.g as f32) * t) as u8,
                    (base.b as f32 + (peak.b as f32 - base.b as f32) * t) as u8,
                )
            }
        }
    }

    /// Color specifically assigned to the floating Peak Hold dot
    pub fn peak_color(&self) -> Color {
        match self {
            Self::StockClassic => Color::new(255, 255, 255),
            Self::CyberpunkNeon => Color::new(255, 255, 255),
            Self::ThermalFire => Color::new(255, 255, 200),
            Self::OceanicIce => Color::new(230, 255, 255),
            Self::CustomGradient { peak, .. } => *peak,
        }
    }

    /// Band specific tint for classic multi-color equalizer columns
    pub fn band_color(&self, band: usize, height_t: f32) -> Color {
        match self {
            Self::StockClassic => {
                // Stock vendor column colors from low bass to high treble
                const BAND_HUES: [f32; AUDIO_BANDS] = [
                    0.55, // Blue-cyan
                    0.50, // Cyan
                    0.42, // Aqua
                    0.33, // Green
                    0.25, // Yellow-green
                    0.16, // Yellow
                    0.09, // Orange
                    0.03, // Red-orange
                    0.96, // Crimson
                    0.82, // Magenta-violet
                ];
                let hue = BAND_HUES[band.min(AUDIO_BANDS - 1)];
                let brightness = 0.70 + 0.30 * height_t;
                Color::from_hsv(hue, 1.0, brightness)
            }
            _ => self.sample(height_t),
        }
    }
}

pub struct MusicRenderer {
    pub style: VisualizerStyle,
    pub palette: ColorPalette,
    bass_history: Vec<f32>,
    history_idx: usize,
    #[allow(dead_code)]
    matrix_drops: Vec<(usize, usize, f32)>, // (col, row, timer)
}

impl MusicRenderer {
    pub fn new(style: VisualizerStyle, custom_color: Option<Color>) -> Self {
        let palette = if let Some(c) = custom_color {
            ColorPalette::CustomGradient {
                base: c.scale(0.3),
                peak: c,
            }
        } else {
            ColorPalette::StockClassic
        };

        Self {
            style,
            palette,
            bass_history: vec![0.0; 32],
            history_idx: 0,
            matrix_drops: Vec::new(),
        }
    }

    pub fn set_palette(&mut self, palette: ColorPalette) {
        self.palette = palette;
    }

    pub fn render(&mut self, analyzer: &SpectrumAnalyzer, t: f32) -> [Color; TOTAL_LEDS] {
        let mut frame = [Color::BLACK; TOTAL_LEDS];
        let levels = analyzer.levels();
        let peaks = analyzer.peaks();

        // Update circular bass history for wave style
        self.bass_history[self.history_idx] = analyzer.bass;
        self.history_idx = (self.history_idx + 1) % self.bass_history.len();

        match self.style {
            VisualizerStyle::Spectrum => {
                // True audiobar.txt column stages: keys light from stage 0 (bottom) to stage 5 (top)
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = key.audio_band.min(AUDIO_BANDS - 1);
                    let level = levels[band];
                    let peak = peaks[band];

                    // Height ratio from 0.0 to 1.0 based on stage (0..5)
                    let stage_ratio = (key.stage as f32) / 5.0;
                    let peak_ratio = (peak * 5.0).round() / 5.0;

                    if stage_ratio <= level * 1.05 {
                        let col = self.palette.band_color(band, stage_ratio);
                        frame[key.hw_idx] = col;
                    } else if (stage_ratio - peak_ratio).abs() < 0.12 && peak > 0.08 {
                        // Peak Hold indicator key
                        frame[key.hw_idx] = self.palette.peak_color();
                    } else {
                        // Subtle ambient key backlight
                        frame[key.hw_idx] = Color::new(2, 4, 8);
                    }
                }
            }

            VisualizerStyle::Pulse => {
                let energy = (0.05 + 0.95 * analyzer.bass.powf(1.4)).clamp(0.0, 1.0);
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let cx = key.rect.center_x() - 280.0;
                    let cy = key.rect.center_y() - 140.0;
                    let dist = (cx * cx + cy * cy).sqrt() / 250.0;

                    let radius = energy * 1.2;
                    if dist <= radius {
                        let t_grad = 1.0 - (dist / radius).clamp(0.0, 1.0);
                        let col = self.palette.sample(t_grad);
                        frame[key.hw_idx] = col;
                    } else {
                        frame[key.hw_idx] = Color::new(2, 4, 8);
                    }
                }
            }

            VisualizerStyle::Wave => {
                let count = self.bass_history.len();
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let norm_x = (key.rect.center_x() / 560.0).clamp(0.0, 1.0);
                    let hist_offset = (norm_x * (count as f32 - 1.0)) as usize;
                    let idx = (self.history_idx + count - 1 - hist_offset) % count;
                    let val = self.bass_history[idx];

                    let norm_y = (key.rect.center_y() - 40.0) / 180.0;
                    let height = (1.0 - norm_y).clamp(0.0, 1.0);

                    if height <= val * 1.1 {
                        let col = self.palette.sample(val);
                        frame[key.hw_idx] = col;
                    } else {
                        frame[key.hw_idx] = Color::new(2, 4, 8);
                    }
                }
            }

            VisualizerStyle::Passion => {
                // High energy flame wave jumping across keys
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = key.audio_band.min(AUDIO_BANDS - 1);
                    let level = levels[band];
                    let norm_y = (key.rect.center_y() - 40.0) / 180.0;
                    let flame = (level * 1.3 - norm_y * 0.4 + (t * 6.0 + key.col as f32).sin() * 0.15).clamp(0.0, 1.0);

                    if flame > 0.15 {
                        let col = self.palette.sample(flame);
                        frame[key.hw_idx] = col;
                    } else {
                        frame[key.hw_idx] = Color::new(2, 4, 8);
                    }
                }
            }

            VisualizerStyle::Matrix => {
                // Digital rain drops triggered by audio transients
                for key in KEYS {
                    if key.hw_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = key.audio_band.min(AUDIO_BANDS - 1);
                    let level = levels[band];
                    let stage_ratio = (key.stage as f32) / 5.0;

                    let drop_phase = (t * 8.0 + (band as f32) * 1.7).fract();
                    let drop_dist = (stage_ratio - (1.0 - drop_phase)).abs();

                    if drop_dist < 0.22 && level > 0.10 {
                        let intensity = (1.0 - drop_dist / 0.22) * level;
                        let col = self.palette.sample(intensity);
                        frame[key.hw_idx] = col;
                    } else {
                        frame[key.hw_idx] = Color::new(1, 3, 5);
                    }
                }
            }
        }

        frame
    }
}
