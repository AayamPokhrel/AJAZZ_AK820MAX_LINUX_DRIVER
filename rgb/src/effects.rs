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
    Static = 1,
    Spectrum = 2,
    Breathing = 3,
    Custom = 4,
    Windmill = 5,
    SineWave = 6,
    Screen = 7,
    TopBottom = 8,
    Galloping = 9,
    Rain = 10,
    RippleLr = 11,
    RippleUd = 12,
    Cross = 13,
    KeyBreathing = 14,
    Starry = 15,
    Ejection = 16,
    Kaleidoscope = 17,
    Music = 18,
    Scroll = 19,
    Off = 20,
}

impl EffectMode {
    pub fn mode_id(self) -> u8 {
        match self {
            Self::Static => 1,
            Self::Spectrum => 2,
            Self::Breathing => 3,
            Self::Custom => 4,
            Self::Windmill => 5,
            Self::SineWave => 6,
            Self::Screen => 7,
            Self::TopBottom => 8,
            Self::Galloping => 9,
            Self::Rain => 10,
            Self::RippleLr => 11,
            Self::RippleUd => 11,
            Self::Cross => 12,
            Self::KeyBreathing => 13,
            Self::Starry => 14,
            Self::Ejection => 15,
            Self::Kaleidoscope => 16,
            Self::Music => 17,
            Self::Scroll => 18,
            Self::Off => 0,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "static" | "fixed" | "constant" | "fixedon" => Some(Self::Static),
            "spectrum" | "rainbow" => Some(Self::Spectrum),
            "breathing" | "respire" | "breathe" => Some(Self::Breathing),
            "custom" | "perkey" | "selfdefine" => Some(Self::Custom),
            "windmill" | "rotatingwindmill" => Some(Self::Windmill),
            "wave" | "sinewave" | "sine" => Some(Self::SineWave),
            "screen" | "neon" | "dazzle" | "neonstream" => Some(Self::Screen),
            "topbottom" | "waterfall" | "colorfulwaterfall" => Some(Self::TopBottom),
            "galloping" | "storm" | "rotatingstorm" => Some(Self::Galloping),
            "rain" | "raindrops" => Some(Self::Rain),
            "ripple" | "ripplelr" | "ripples" | "ripplesshining" => Some(Self::RippleLr),
            "rippleud" => Some(Self::RippleUd),
            "cross" | "blossom" | "blossoming" => Some(Self::Cross),
            "keybreathing" | "reactive" | "reaction" => Some(Self::KeyBreathing),
            "starry" | "stars" | "starstwinkle" => Some(Self::Starry),
            "ejection" | "flashaway" => Some(Self::Ejection),
            "kaleidoscope" | "rainbowwheel" => Some(Self::Kaleidoscope),
            "music" | "audioreactive" => Some(Self::Music),
            "scroll" | "shadowdisappear" | "retinuescanning" => Some(Self::Scroll),
            "off" | "none" | "lightsoff" => Some(Self::Off),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Spectrum => "spectrum",
            Self::Breathing => "breathing",
            Self::Custom => "custom",
            Self::Windmill => "windmill",
            Self::SineWave => "sine-wave",
            Self::Screen => "neon",
            Self::TopBottom => "waterfall",
            Self::Galloping => "storm",
            Self::Rain => "rain",
            Self::RippleLr => "ripple-lr",
            Self::RippleUd => "ripple-ud",
            Self::Cross => "blossom",
            Self::KeyBreathing => "reactive",
            Self::Starry => "starry",
            Self::Ejection => "ejection",
            Self::Kaleidoscope => "kaleidoscope",
            Self::Music => "music",
            Self::Scroll => "scroll",
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
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = c;
                    }
                }
            }
            EffectMode::Breathing => {
                let cycle = (t * 1.5).sin() * 0.5 + 0.5;
                let c = color.unwrap_or(Color::from_hsv(t * 0.1, 1.0, 1.0)).scale(cycle * bri);
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        frame[key.hw_idx] = c;
                    }
                }
            }
            EffectMode::Spectrum => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let hue = t * 0.2 + (key.col as f32) * 0.05;
                        frame[key.hw_idx] = Color::from_hsv(hue, 1.0, bri);
                    }
                }
            }
            EffectMode::SineWave => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let offset = (key.rect.center_x() / 600.0) * std::f32::consts::PI * 2.0;
                        let wave = ((t * 3.0 + offset).sin() * 0.5 + 0.5) * bri;
                        frame[key.hw_idx] = color.unwrap_or_else(|| Color::from_hsv(t * 0.1 + offset * 0.2, 1.0, 1.0)).scale(wave);
                    }
                }
            }
            EffectMode::Off => {
                // Already all black
            }
            _ => {
                // Rainbow wave fallback
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let u = key.rect.center_x() / 600.0;
                        frame[key.hw_idx] = Color::from_hsv(t * 0.2 + u, 1.0, bri);
                    }
                }
            }
        }

        frame
    }
}
