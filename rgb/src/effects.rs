use crate::color::{Color, PaletteColor};
use crate::layout::{KEYS, TOTAL_LEDS};
use crate::typing::{TypingEngine, TypingMode};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Direction {
    Right = 0,
    Left = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectMode {
    // Hardware modes verified against kb-k600t.xml & OemDrv.exe
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
    Custom = 24,
    Off = 0,

    // Software Live Interactive Typing Modes (v1.2.0)
    TypingSplash = 101,
    TypingRoulette = 102,
    TypingShockwave = 103,
    TypingHeatmap = 104,
    TypingMatrix = 105,
    TypingLaser = 106,
    TypingSparks = 107,
    TypingAurora = 108,

    // Backward compatibility alias for Screen
    #[serde(alias = "Screen")]
    NeonStream = 109,
}

impl EffectMode {
    /// Return the exact hardware effect ID expected by the keyboard firmware (kb-k600t.xml)
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
            Self::Custom => 24,
            Self::Off => 0,

            // Live software interactive typing modes use hardware live stream mode (22)
            Self::TypingSplash
            | Self::TypingRoulette
            | Self::TypingShockwave
            | Self::TypingHeatmap
            | Self::TypingMatrix
            | Self::TypingLaser
            | Self::TypingSparks
            | Self::TypingAurora => 22,

            Self::NeonStream => 5,
        }
    }

    pub fn is_typing_reactive(self) -> bool {
        matches!(
            self,
            Self::TypingSplash
                | Self::TypingRoulette
                | Self::TypingShockwave
                | Self::TypingHeatmap
                | Self::TypingMatrix
                | Self::TypingLaser
                | Self::TypingSparks
                | Self::TypingAurora
                | Self::RippleLr
                | Self::RippleUd
                | Self::Ejection
                | Self::Cross
                | Self::KeyBreathing
        )
    }

    pub fn is_software_stream(self) -> bool {
        matches!(
            self,
            Self::TypingSplash
                | Self::TypingRoulette
                | Self::TypingShockwave
                | Self::TypingHeatmap
                | Self::TypingMatrix
                | Self::TypingLaser
                | Self::TypingSparks
                | Self::TypingAurora
                | Self::Custom
        )
    }

    pub fn to_typing_mode(self) -> Option<TypingMode> {
        match self {
            Self::TypingSplash => Some(TypingMode::RandomGradientSplash),
            Self::TypingRoulette => Some(TypingMode::ChaosRoulette),
            Self::TypingShockwave => Some(TypingMode::ShockwaveRipple),
            Self::TypingHeatmap => Some(TypingMode::ThermalHeatmap),
            Self::TypingMatrix => Some(TypingMode::MatrixCodeRain),
            Self::TypingLaser => Some(TypingMode::CrossfireLaser),
            Self::TypingSparks => Some(TypingMode::StarlightSparks),
            Self::TypingAurora => Some(TypingMode::CosmicAurora),
            _ => None,
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
            "screen" | "neon" | "dazzle" | "neonstream" => Some(Self::NeonStream),
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

            // Interactive typing modes
            "typingsplash" | "splash" | "randomgradient" | "gradientsplash" => Some(Self::TypingSplash),
            "typingroulette" | "roulette" | "chaos" | "chaosfx" => Some(Self::TypingRoulette),
            "typingshockwave" | "shockwave" => Some(Self::TypingShockwave),
            "typingheatmap" | "heatmap" | "thermal" | "fire" => Some(Self::TypingHeatmap),
            "typingmatrix" | "matrix" | "coderain" => Some(Self::TypingMatrix),
            "typinglaser" | "laser" | "crossfire" => Some(Self::TypingLaser),
            "typingsparks" | "sparks" | "starlight" => Some(Self::TypingSparks),
            "typingaurora" | "aurora" => Some(Self::TypingAurora),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::RippleLr => "ripple-lr",
            Self::Starry => "starry",
            Self::RippleUd => "ripple-ud",
            Self::Ejection => "ejection",
            Self::Spectrum => "spectrum",
            Self::Kaleidoscope => "kaleidoscope",
            Self::Static => "static",
            Self::Breathing => "breathing",
            Self::Cross => "blossom",
            Self::KeyBreathing => "reactive",
            Self::Scroll => "scroll",
            Self::TopBottom => "waterfall",
            Self::Rain => "rain",
            Self::Windmill => "windmill",
            Self::SineWave => "sine-wave",
            Self::Galloping => "storm",
            Self::Music => "music",
            Self::Custom => "custom",
            Self::Off => "off",
            Self::NeonStream => "neon",

            Self::TypingSplash => "typing-splash",
            Self::TypingRoulette => "typing-roulette",
            Self::TypingShockwave => "typing-shockwave",
            Self::TypingHeatmap => "typing-heatmap",
            Self::TypingMatrix => "typing-matrix",
            Self::TypingLaser => "typing-laser",
            Self::TypingSparks => "typing-sparks",
            Self::TypingAurora => "typing-aurora",
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
            mode: EffectMode::Spectrum,
            brightness: 5,
            speed: 4,
            direction: Direction::Right,
            palette_color: PaletteColor::Rainbow,
            custom_color: None,
        }
    }
}

// Global preview typing engine for GUI animation
static PREVIEW_TYPING_ENGINE: Mutex<Option<(TypingMode, TypingEngine, f32)>> = Mutex::new(None);

/// Software frame generator for rendering frames to 108-LED matrix.
pub struct FrameRenderer;

impl FrameRenderer {
    /// Render a single animation frame for a given effect and time in seconds.
    pub fn render(mode: EffectMode, t: f32, color: Option<Color>, brightness: f32) -> [Color; TOTAL_LEDS] {
        let mut frame = [Color::BLACK; TOTAL_LEDS];
        let bri = brightness.clamp(0.0, 1.0);

        // Check if this is an interactive software typing mode
        if let Some(typing_mode) = mode.to_typing_mode() {
            let mut guard = PREVIEW_TYPING_ENGINE.lock().unwrap();
            let entry = guard.get_or_insert_with(|| {
                (typing_mode, TypingEngine::new(typing_mode), t)
            });

            if entry.0 != typing_mode {
                entry.0 = typing_mode;
                entry.1.set_mode(typing_mode);
                entry.2 = t;
            }

            let dt = (t - entry.2).clamp(0.001, 0.05);
            entry.2 = t;

            // Idle simulation: simulate a soft keypress pulse periodically when idle
            let idle_step = (t * 2.2) as u64;
            if (t * 2.2).fract() < 0.04 && idle_step % 2 == 0 {
                let idx = ((t * 7.0) as usize) % KEYS.len();
                let key = &KEYS[idx];
                entry.1.on_key_down(key.hw_idx, key.row, key.col, key.rect.center_x(), key.rect.center_y());
            }

            let rendered = entry.1.update(dt);
            for i in 0..TOTAL_LEDS {
                frame[i] = rendered[i].scale(bri);
            }
            return frame;
        }

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
            EffectMode::Spectrum | EffectMode::NeonStream => {
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
            EffectMode::RippleLr => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let dist = (key.rect.center_x() - 300.0).abs();
                        let wave = ((dist * 0.04 - t * 4.0).sin() * 0.5 + 0.5) * bri;
                        frame[key.hw_idx] = color.unwrap_or(Color::CYAN).scale(wave);
                    }
                }
            }
            EffectMode::RippleUd => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let dist = (key.rect.center_y() - 120.0).abs();
                        let wave = ((dist * 0.05 - t * 4.0).sin() * 0.5 + 0.5) * bri;
                        frame[key.hw_idx] = color.unwrap_or(Color::MAGENTA).scale(wave);
                    }
                }
            }
            EffectMode::Starry => {
                for key in KEYS {
                    if key.hw_idx < TOTAL_LEDS {
                        let pseudo_rand = ((key.hw_idx as f32 * 13.37 + t * 2.0).sin() * 0.5 + 0.5).powf(4.0) * bri;
                        frame[key.hw_idx] = color.unwrap_or(Color::WHITE).scale(pseudo_rand);
                    }
                }
            }
            EffectMode::Off => {
                // Already all black
            }
            _ => {
                // Vibrant wave fallback
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

    /// Forward a user key press from the GUI to the preview engine
    pub fn on_preview_key_press(hw_idx: usize, row: usize, col: usize, x: f32, y: f32) {
        let mut guard = PREVIEW_TYPING_ENGINE.lock().unwrap();
        if let Some((_, ref mut engine, _)) = *guard {
            engine.on_key_down(hw_idx, row, col, x, y);
        }
    }
}
