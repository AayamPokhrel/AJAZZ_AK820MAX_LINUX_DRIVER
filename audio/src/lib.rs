pub mod capture;
pub mod spectrum;
pub mod visualizer;

pub use capture::{AudioCapture, AudioError, AudioSource, AudioSourceInfo};
pub use spectrum::{AUDIO_BANDS, BAND_LABELS, FFT_SIZE, SpectrumAnalyzer};
pub use visualizer::{
    ColorMode, ColorPalette, MusicRenderer, VisualizerConfig, VisualizerStyle, ZoneDirection,
};

pub const F_LO: f32 = 25.0;
pub const F_HI: f32 = 20000.0;
