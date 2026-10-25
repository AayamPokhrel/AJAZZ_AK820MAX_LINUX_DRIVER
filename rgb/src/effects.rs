use crate::color::{Color, PaletteColor};
use crate::layout::{KEYS, TOTAL_LEDS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Direction {
    Right = 0,
    Left = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectMode {
    RippleLr = 1,
    Starry = 2,
    RippleUd = 3,
    Ejection = 4,
    Spectrum = 5,
    Kaleidoscope = 6,
    Static = 7,
    Breathing = 8,
    Cross = 9,
    KeyBreathing = 11,
    Scroll = 12,
    TopBottom = 15,
    Rain = 16,
    Windmill = 17,
    SineWave = 18,
    Galloping = 19,
    Music = 22,
    Screen = 23,
    Custom = 24,
    Off = 25,
}

impl EffectMode {
    pub fn mode_id(self) -> u8 {
        match self {
            Self::RippleLr => 1,
            Self::Starry => 2,
            Self::RippleUd => 3,
            Self::Ejection => 4,
            Self::Spectrum => 5,
            Self::Kaleidoscope => 6,
            Self::Static => 7,
            Self::Breathing => 8,
            Self::Cross => 9,
            Self::KeyBreathing => 11,
            Self::Scroll => 12,
            Self::TopBottom => 15,
            Self::Rain => 16,
            Self::Windmill => 17,
            Self::SineWave => 18,
            Self::Galloping => 19,
            Self::Music => 22,
            Self::Screen => 23,
            Self::Custom => 24,
            Self::Off => 20,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "static" | "fixed" | "constant" => Some(Self::Static),
            "breathing" | "respire" | "breathe" => Some(Self::Breathing),
            "wave" | "sinewave" | "sine" => Some(Self::SineWave),
            "rainbow" | "spectrum" => Some(Self::Spectrum),
            "ripple" | "ripplelr" => Some(Self::RippleLr),
            "rippleud" => Some(Self::RippleUd),
            "starry" | "stars" => Some(Self::Starry),
            "ejection" => Some(Self::Ejection),
            "kaleidoscope" => Some(Self::Kaleidoscope),
            "cross" => Some(Self::Cross),
            "keybreathing" | "reactive" => Some(Self::KeyBreathing),
            "scroll" => Some(Self::Scroll),
            "topbottom" => Some(Self::TopBottom),
            "rain" | "raindrops" => Some(Self::Rain),
            "windmill" => Some(Self::Windmill),
            "galloping" => Some(Self::Galloping),
            "music" => Some(Self::Music),
            "custom" | "perkey" => Some(Self::Custom),
            "off" | "none" => Some(Self::Off),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Breathing => "breathing",
            Self::SineWave => "sine-wave",
            Self::Spectrum => "spectrum",
            Self::RippleLr => "ripple-lr",
            Self::RippleUd => "ripple-ud",
            Self::Starry => "starry",
            Self::Ejection => "ejection",
            Self::Kaleidoscope => "kaleidoscope",
            Self::Cross => "cross",
            Self::KeyBreathing => "reactive",
            Self::Scroll => "scroll",
            Self::TopBottom => "top-bottom",
            Self::Rain => "rain",
            Self::Windmill => "windmill",
            Self::Galloping => "galloping",
            Self::Music => "music",
            Self::Screen => "screen",
            Self::Custom => "custom",
            Self::Off => "off",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectSettings {
    pub mode: EffectMode,
    /// 1..=5
    pub brightness: u8,
    /// 1..=5
    pub speed: u8,
    pub direction: Direction,
    pub palette_color: PaletteColor,
    pub custom_color: Option<Color>,
}

impl Default for EffectSettings {
    fn default() -> Self {
        Self {
            mode: EffectMode::Static,
            brightness: 5,
            speed: 4,
            direction: Direction::Right,
            palette_color: PaletteColor::Rainbow,
            custom_color: None,
        }
    }
}

/// Software frame generator for rendering frames to 108-LED matrix.
pub struct FrameRenderer;

impl FrameRenderer {
    /// Render a single animation frame for a given effect and time in seconds.
    pub fn render(mode: EffectMode, t: f32, color: Option<Color>, brightness: f32) -> [Color; TOTAL_LEDS] {
        let mut frame = [Color::BLACK; TOTAL_LEDS];
        let bri = brightness.clamp(0.0, 1.0);

        match mode {
            EffectMode::Static => {
                let c = color.unwrap_or(Color::WHITE).scale(bri);
                for key in KEYS {
                    if key.led_idx < TOTAL_LEDS {
                        frame[key.led_idx] = c;
                    }
                }
            }
            EffectMode::Breathing => {
                let cycle = (t * 1.5).sin() * 0.5 + 0.5;
                let c = color.unwrap_or(Color::from_hsv(t * 0.1, 1.0, 1.0)).scale(cycle * bri);
                for key in KEYS {
                    if key.led_idx < TOTAL_LEDS {
                        frame[key.led_idx] = c;
                    }
                }
            }
            EffectMode::Spectrum => {
                for key in KEYS {
                    if key.led_idx < TOTAL_LEDS {
                        let hue = t * 0.2 + (key.col as f32) * 0.05;
                        frame[key.led_idx] = Color::from_hsv(hue, 1.0, bri);
                    }
                }
            }
            EffectMode::SineWave => {
                for key in KEYS {
                    if key.led_idx < TOTAL_LEDS {
                        let offset = (key.rect.center_x() / 600.0) * std::f32::consts::PI * 2.0;
                        let wave = ((t * 3.0 + offset).sin() * 0.5 + 0.5) * bri;
                        frame[key.led_idx] = color.unwrap_or_else(|| Color::from_hsv(t * 0.1 + offset * 0.2, 1.0, 1.0)).scale(wave);
                    }
                }
            }
            EffectMode::Off => {
                // Already all black
            }
            _ => {
                // Fallback to rainbow wave
                for key in KEYS {
                    if key.led_idx < TOTAL_LEDS {
                        let u = key.rect.center_x() / 600.0;
                        frame[key.led_idx] = Color::from_hsv(t * 0.2 + u, 1.0, bri);
                    }
                }
            }
        }

        frame
    }
}
