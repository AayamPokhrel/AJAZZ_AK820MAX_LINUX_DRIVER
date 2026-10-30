use crate::color::{Color, PaletteColor};
use crate::effects::{Direction, EffectMode, EffectSettings};
use crate::layout::TOTAL_LEDS;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightingConfig {
    pub mode: EffectMode,
    pub brightness: u8,
    pub speed: u8,
    pub direction: Direction,
    pub palette_color: PaletteColor,
    pub custom_color: Option<Color>,
}

impl Default for LightingConfig {
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

impl From<EffectSettings> for LightingConfig {
    fn from(s: EffectSettings) -> Self {
        Self {
            mode: s.mode,
            brightness: s.brightness,
            speed: s.speed,
            direction: s.direction,
            palette_color: s.palette_color,
            custom_color: s.custom_color,
        }
    }
}

impl From<LightingConfig> for EffectSettings {
    fn from(c: LightingConfig) -> Self {
        Self {
            mode: c.mode,
            brightness: c.brightness,
            speed: c.speed,
            direction: c.direction,
            palette_color: c.palette_color,
            custom_color: c.custom_color,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MusicVisualizerSavedConfig {
    pub style: String,
    pub color_mode: String,
    pub custom_tint: Option<String>,
    pub sensitivity: f32,
    pub noise_gate: f32,
    pub decay_speed: f32,
    pub peak_hold: bool,
    pub target_fps: u32,
    pub morph_speed: f32,
    pub audio_source: String,
}

impl Default for MusicVisualizerSavedConfig {
    fn default() -> Self {
        Self {
            style: "Spectrum".to_string(),
            color_mode: "Classic".to_string(),
            custom_tint: None,
            sensitivity: 1.2,
            noise_gate: 0.04,
            decay_speed: 2.8,
            peak_hold: true,
            target_fps: 60,
            morph_speed: 1.5,
            audio_source: "System Desktop Audio (Loopback)".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomCanvasSavedConfig {
    pub colors: Vec<String>,
    pub is_active: bool,
    pub brightness: u8,
}

impl Default for CustomCanvasSavedConfig {
    fn default() -> Self {
        Self {
            colors: vec!["#00F2FE".to_string(); TOTAL_LEDS],
            is_active: false,
            brightness: 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSavedConfig {
    pub sleep_minutes: u8,
    pub debounce_ms: u8,
}

impl Default for DeviceSavedConfig {
    fn default() -> Self {
        Self {
            sleep_minutes: 5,
            debounce_ms: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentConfig {
    pub active_category: String,
    pub lighting: LightingConfig,
    pub music: MusicVisualizerSavedConfig,
    pub canvas: CustomCanvasSavedConfig,
    pub device: DeviceSavedConfig,
}

impl Default for PersistentConfig {
    fn default() -> Self {
        Self {
            active_category: "effect".to_string(),
            lighting: LightingConfig::default(),
            music: MusicVisualizerSavedConfig::default(),
            canvas: CustomCanvasSavedConfig::default(),
            device: DeviceSavedConfig::default(),
        }
    }
}

impl PersistentConfig {
    pub fn config_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
            })?;
        Some(base.join("ajazz-ak820").join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<Self>(&content) {
                        return cfg;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let temp_path = path.with_extension("tmp");
            fs::write(&temp_path, json)?;
            fs::rename(temp_path, path)?;
        }
        Ok(())
    }
}
