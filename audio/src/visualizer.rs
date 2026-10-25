use crate::spectrum::{AUDIO_BANDS, SpectrumAnalyzer};
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::{KEYS, TOTAL_LEDS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualizerStyle {
    Spectrum,
    Pulse,
    Wave,
    SoftDance,
    Passion,
}

impl VisualizerStyle {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "spectrum" | "bars" | "equalizer" => Some(Self::Spectrum),
            "pulse" | "beat" => Some(Self::Pulse),
            "wave" | "ripple" => Some(Self::Wave),
            "softdance" | "dance" | "soft" => Some(Self::SoftDance),
            "passion" | "rock" => Some(Self::Passion),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Spectrum => "spectrum",
            Self::Pulse => "pulse",
            Self::Wave => "wave",
            Self::SoftDance => "soft-dance",
            Self::Passion => "passion",
        }
    }
}

pub struct MusicRenderer {
    pub style: VisualizerStyle,
    pub custom_color: Option<Color>,
    bass_history: Vec<f32>,
    history_idx: usize,
}

impl MusicRenderer {
    pub fn new(style: VisualizerStyle, custom_color: Option<Color>) -> Self {
        Self {
            style,
            custom_color,
            bass_history: vec![0.0; 32],
            history_idx: 0,
        }
    }

    pub fn render(&mut self, analyzer: &SpectrumAnalyzer, t: f32) -> [Color; TOTAL_LEDS] {
        let mut frame = [Color::BLACK; TOTAL_LEDS];
        let levels = analyzer.levels();

        // Update circular bass history for wave style
        self.bass_history[self.history_idx] = analyzer.bass;
        self.history_idx = (self.history_idx + 1) % self.bass_history.len();

        match self.style {
            VisualizerStyle::Spectrum => {
                // Columns rising from bottom to top across the 10 bands
                for key in KEYS {
                    if key.led_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = key.audio_band.min(AUDIO_BANDS - 1);
                    let level = levels[band]; // 0.0..=1.0

                    // Y normalized 0.0 (top) to 1.0 (bottom)
                    let norm_y = (key.rect.center_y() - 40.0) / 180.0;
                    let height_from_bottom = (1.0 - norm_y).clamp(0.0, 1.0);

                    if height_from_bottom <= level * 1.05 {
                        let col = if let Some(base_col) = self.custom_color {
                            base_col.scale(0.1 + 0.9 * level)
                        } else {
                            // Green at bottom (0.33) -> Yellow -> Red at top (0.0)
                            let hue = 0.33 * (1.0 - height_from_bottom);
                            Color::from_hsv(hue, 1.0, 0.95)
                        };
                        frame[key.led_idx] = col;
                    } else {
                        // Ambient dim glow
                        frame[key.led_idx] = Color::new(2, 4, 8);
                    }
                }
            }

            VisualizerStyle::Pulse => {
                let val = (0.08 + 0.92 * analyzer.bass.powf(1.6)).clamp(0.0, 1.0);
                let base_hue = (t * 0.1).fract();

                for key in KEYS {
                    if key.led_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let norm_x = (key.rect.center_x() - 30.0) / 520.0;
                    let col = if let Some(base_col) = self.custom_color {
                        base_col.scale(val)
                    } else {
                        Color::from_hsv(base_hue + norm_x * 0.25, 0.95, val)
                    };
                    frame[key.led_idx] = col;
                }
            }

            VisualizerStyle::Wave => {
                let center_x = 280.0;
                let center_y = 130.0;

                for key in KEYS {
                    if key.led_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let dx = key.rect.center_x() - center_x;
                    let dy = (key.rect.center_y() - center_y) * 2.2;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let norm_dist = (dist / 320.0).clamp(0.0, 1.0);

                    let hist_len = self.bass_history.len();
                    let sample_offset = (norm_dist * (hist_len as f32 - 1.0)) as usize;
                    let lookup_idx = (self.history_idx + hist_len - sample_offset) % hist_len;
                    let energy = self.bass_history[lookup_idx];

                    let bri = (0.06 + 0.94 * energy.powf(1.4)).clamp(0.0, 1.0);
                    let col = if let Some(base_col) = self.custom_color {
                        base_col.scale(bri)
                    } else {
                        Color::from_hsv(t * 0.15 + norm_dist * 0.7, 0.95, bri)
                    };
                    frame[key.led_idx] = col;
                }
            }

            VisualizerStyle::SoftDance => {
                let flow = (t * 0.8).fract();
                for key in KEYS {
                    if key.led_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let norm_x = (key.rect.center_x() - 30.0) / 520.0;
                    let band = key.audio_band.min(AUDIO_BANDS - 1);
                    let level = levels[band];

                    let wave = ((norm_x * 3.0 - flow * 3.0).sin() * 0.5 + 0.5) * level;
                    let bri = (0.1 + 0.9 * wave).clamp(0.0, 1.0);

                    let col = if let Some(base_col) = self.custom_color {
                        base_col.scale(bri)
                    } else {
                        Color::from_hsv(flow + norm_x * 0.3, 0.85, bri)
                    };
                    frame[key.led_idx] = col;
                }
            }

            VisualizerStyle::Passion => {
                let is_flash = analyzer.bass > 0.75;
                for key in KEYS {
                    if key.led_idx >= TOTAL_LEDS {
                        continue;
                    }
                    let band = key.audio_band.min(AUDIO_BANDS - 1);
                    let level = levels[band];

                    let bri = if is_flash {
                        1.0
                    } else {
                        (0.05 + 0.95 * level.powf(2.0)).clamp(0.0, 1.0)
                    };

                    let col = if is_flash {
                        Color::new(255, 230, 200)
                    } else if let Some(base_col) = self.custom_color {
                        base_col.scale(bri)
                    } else {
                        let hue = match band {
                            0..=2 => 0.0,   // Red bass
                            3..=6 => 0.83,  // Magenta / Purple mids
                            _ => 0.55,      // Cyan highs
                        };
                        Color::from_hsv(hue, 1.0, bri)
                    };
                    frame[key.led_idx] = col;
                }
            }
        }

        frame
    }
}
