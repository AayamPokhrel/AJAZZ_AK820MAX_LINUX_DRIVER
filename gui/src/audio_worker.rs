use ajazz_audio::capture::{AudioCapture, AudioSource};
use ajazz_audio::spectrum::{SpectrumAnalyzer, FFT_SIZE};
use ajazz_audio::visualizer::{ColorPalette, MusicRenderer, VisualizerStyle};
use ajazz_hid::AjazzDevice;
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::TOTAL_LEDS;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{error, info};

#[derive(Clone, Debug)]
pub struct AudioVisualizerState {
    pub is_active: bool,
    pub levels: [f32; 10],
    pub peaks: [f32; 10],
    pub bass: f32,
    pub frame: [Color; TOTAL_LEDS],
    pub error: Option<String>,
}

impl Default for AudioVisualizerState {
    fn default() -> Self {
        Self {
            is_active: false,
            levels: [0.0; 10],
            peaks: [0.0; 10],
            bass: 0.0,
            frame: [Color::BLACK; TOTAL_LEDS],
            error: None,
        }
    }
}

#[derive(Clone, Debug)]
struct WorkerConfig {
    pub style: VisualizerStyle,
    pub palette: ColorPalette,
    pub custom_color: Option<Color>,
    pub sensitivity: f32,
    pub noise_gate: f32,
    pub fps: u32,
}

pub struct AudioVisualizerWorker {
    state: Arc<Mutex<AudioVisualizerState>>,
    config: Arc<Mutex<WorkerConfig>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl AudioVisualizerWorker {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(AudioVisualizerState::default()));
        let config = Arc::new(Mutex::new(WorkerConfig {
            style: VisualizerStyle::Spectrum,
            palette: ColorPalette::StockClassic,
            custom_color: None,
            sensitivity: 1.2,
            noise_gate: 0.05,
            fps: 60,
        }));
        let running = Arc::new(AtomicBool::new(false));

        Self {
            state,
            config,
            running,
            handle: None,
        }
    }

    pub fn state(&self) -> AudioVisualizerState {
        self.state.lock().unwrap().clone()
    }

    pub fn is_active(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn update_config(
        &self,
        style: VisualizerStyle,
        palette: ColorPalette,
        custom_color: Option<Color>,
        sensitivity: f32,
        noise_gate: f32,
        fps: u32,
    ) {
        if let Ok(mut cfg) = self.config.lock() {
            cfg.style = style;
            cfg.palette = palette;
            cfg.custom_color = custom_color;
            cfg.sensitivity = sensitivity;
            cfg.noise_gate = noise_gate;
            cfg.fps = fps;
        }
    }

    pub fn start(
        &mut self,
        style: VisualizerStyle,
        palette: ColorPalette,
        custom_color: Option<Color>,
        sensitivity: f32,
        noise_gate: f32,
        fps: u32,
        source: Option<AudioSource>,
        direct_device: Option<Arc<AjazzDevice>>,
    ) {
        if self.running.load(Ordering::SeqCst) {
            self.update_config(style, palette, custom_color, sensitivity, noise_gate, fps);
            return;
        }

        self.update_config(style, palette, custom_color, sensitivity, noise_gate, fps);
        self.running.store(true, Ordering::SeqCst);

        let state_clone = Arc::clone(&self.state);
        let config_clone = Arc::clone(&self.config);
        let running_clone = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            info!("Starting native GUI Audio Capture & Visualizer Worker thread...");

            let capture = match AudioCapture::new(source) {
                Ok(c) => c,
                Err(e) => {
                    error!("Audio capture error: {}", e);
                    let mut st = state_clone.lock().unwrap();
                    st.error = Some(format!("Audio capture error: {}", e));
                    st.is_active = false;
                    running_clone.store(false, Ordering::SeqCst);
                    return;
                }
            };

            info!(
                "Audio capture source connected: '{}' @ {} Hz",
                capture.source_name(),
                capture.sample_rate()
            );

            let initial_cfg = config_clone.lock().unwrap().clone();
            let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), initial_cfg.sensitivity);
            analyzer.set_noise_gate(initial_cfg.noise_gate);

            let mut renderer = MusicRenderer::new(initial_cfg.style, initial_cfg.custom_color);
            renderer.set_palette(initial_cfg.palette);

            if let Some(ref dev) = direct_device {
                if let Err(e) = dev.enter_live_music_mode() {
                    error!("Direct device live music mode initialization failed: {}", e);
                }
            }

            {
                let mut st = state_clone.lock().unwrap();
                st.is_active = true;
                st.error = None;
            }

            let mut last_instant = Instant::now();

            while running_clone.load(Ordering::SeqCst) {
                let start_tick = Instant::now();
                let dt = last_instant.elapsed().as_secs_f32().max(0.001);
                last_instant = start_tick;

                // Sync dynamic config updates
                let cur_fps = if let Ok(cfg) = config_clone.lock() {
                    analyzer.set_sensitivity(cfg.sensitivity);
                    analyzer.set_noise_gate(cfg.noise_gate);
                    renderer.style = cfg.style;
                    renderer.set_palette(cfg.palette.clone());
                    cfg.fps
                } else {
                    60
                };

                let pcm = capture.read_latest(FFT_SIZE);
                if !pcm.is_empty() {
                    analyzer.update_with_dt(&pcm, dt);
                }

                let frame = renderer.render(&analyzer, dt);

                // Stream physically to keyboard if direct connection is active
                if let Some(ref dev) = direct_device {
                    let _ = dev.send_live_frame(&frame);
                }

                // Update shared state for GUI canvas and spectrum analyzer display
                if let Ok(mut st) = state_clone.lock() {
                    st.levels = analyzer.levels();
                    st.peaks = analyzer.peaks();
                    st.bass = analyzer.bass;
                    st.frame = frame;
                }

                let target_interval = Duration::from_micros((1_000_000 / cur_fps.clamp(10, 90) as u64).max(11_111));
                let elapsed = start_tick.elapsed();
                if elapsed < target_interval {
                    thread::sleep(target_interval - elapsed);
                }
            }

            // Cleanup when stopped
            if let Ok(mut st) = state_clone.lock() {
                st.is_active = false;
                st.levels = [0.0; 10];
                st.peaks = [0.0; 10];
                st.bass = 0.0;
                st.frame = [Color::BLACK; TOTAL_LEDS];
            }
            info!("GUI Audio Capture Worker stopped cleanly.");
        });

        self.handle = Some(handle);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for AudioVisualizerWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
