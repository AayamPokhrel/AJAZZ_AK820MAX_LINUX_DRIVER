use ajazz_audio::capture::{AudioCapture, AudioSource};
use ajazz_audio::spectrum::{AUDIO_BANDS, FFT_SIZE, SpectrumAnalyzer};
use ajazz_audio::visualizer::{MusicRenderer, VisualizerConfig};
use ajazz_hid::AjazzDevice;
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::TOTAL_LEDS;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{error, info};

#[derive(Clone, Debug)]
pub struct AudioVisualizerState {
    pub is_active: bool,
    pub levels: [f32; AUDIO_BANDS],
    pub peaks: [f32; AUDIO_BANDS],
    pub bass: f32,
    pub beat_detected: bool,
    pub frame: [Color; TOTAL_LEDS],
    pub error: Option<String>,
}

impl Default for AudioVisualizerState {
    fn default() -> Self {
        Self {
            is_active: false,
            levels: [0.0; AUDIO_BANDS],
            peaks: [0.0; AUDIO_BANDS],
            bass: 0.0,
            beat_detected: false,
            frame: [Color::BLACK; TOTAL_LEDS],
            error: None,
        }
    }
}

pub struct AudioVisualizerWorker {
    state: Arc<Mutex<AudioVisualizerState>>,
    config: Arc<Mutex<VisualizerConfig>>,
    action_trigger: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    hid_handle: Option<JoinHandle<()>>,
}

impl AudioVisualizerWorker {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(AudioVisualizerState::default()));
        let config = Arc::new(Mutex::new(VisualizerConfig::default()));
        let action_trigger = Arc::new(AtomicUsize::new(0));
        let running = Arc::new(AtomicBool::new(false));

        Self {
            state,
            config,
            action_trigger,
            running,
            handle: None,
            hid_handle: None,
        }
    }

    pub fn state(&self) -> AudioVisualizerState {
        self.state.lock().unwrap().clone()
    }

    pub fn is_active(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn update_config(&self, config: VisualizerConfig) {
        if let Ok(mut cfg) = self.config.lock() {
            *cfg = config;
        }
    }

    pub fn shuffle_zones(&self) {
        self.action_trigger.store(1, Ordering::Relaxed);
    }

    pub fn reroll_mosaic(&self) {
        self.action_trigger.store(2, Ordering::Relaxed);
    }

    pub fn start(
        &mut self,
        config: VisualizerConfig,
        source: Option<AudioSource>,
        direct_device: Option<Arc<AjazzDevice>>,
    ) {
        if self.running.load(Ordering::SeqCst) {
            self.update_config(config);
            return;
        }

        self.update_config(config);
        self.running.store(true, Ordering::SeqCst);

        let state_clone = Arc::clone(&self.state);
        let config_clone = Arc::clone(&self.config);
        let trigger_clone = Arc::clone(&self.action_trigger);
        let running_clone = Arc::clone(&self.running);
        let running_hid = Arc::clone(&self.running);

        // Single-slot conflated frame buffer for direct HID writing
        let latest_frame: Arc<(Mutex<Option<[Color; TOTAL_LEDS]>>, std::sync::Condvar)> =
            Arc::new((Mutex::new(None), std::sync::Condvar::new()));
        let frame_producer = Arc::clone(&latest_frame);
        let frame_consumer = Arc::clone(&latest_frame);

        // Dedicated Zero-Latency Hardware HID Writer Thread (if direct HID device provided)
        if let Some(dev) = direct_device {
            let hid_thread = thread::spawn(move || {
                info!("GUI Direct Hardware HID Writer active (Latest-Frame-Wins)");
                while running_hid.load(Ordering::SeqCst) {
                    let frame_opt = {
                        let (lock, cvar) = &*frame_consumer;
                        let mut slot = lock.lock().unwrap();
                        while slot.is_none() && running_hid.load(Ordering::SeqCst) {
                            slot = cvar.wait_timeout(slot, Duration::from_millis(25)).unwrap().0;
                        }
                        slot.take()
                    };

                    if let Some(frame) = frame_opt {
                        let _ = dev.send_live_frame(&frame);
                    }
                }
                info!("GUI Direct Hardware HID Writer stopped cleanly");
            });
            self.hid_handle = Some(hid_thread);
        }

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
            analyzer.set_decay_speed(initial_cfg.decay_speed);
            analyzer.set_peak_hold(initial_cfg.peak_hold);

            let mut renderer = MusicRenderer::with_config(initial_cfg);

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

                // Sync dynamic config updates in-place (<1ms)
                let cur_fps = if let Ok(cfg) = config_clone.lock() {
                    analyzer.set_sensitivity(cfg.sensitivity);
                    analyzer.set_noise_gate(cfg.noise_gate);
                    analyzer.set_decay_speed(cfg.decay_speed);
                    analyzer.set_peak_hold(cfg.peak_hold);
                    renderer.config.style = cfg.style;
                    renderer.config.color_mode = cfg.color_mode.clone();
                    renderer.config.morph_speed = cfg.morph_speed;
                    cfg.fps
                } else {
                    60
                };

                // Check action triggers
                let action = trigger_clone.swap(0, Ordering::Relaxed);
                if action == 1 {
                    renderer.shuffle_zone_gradient();
                } else if action == 2 {
                    renderer.reroll_mosaic();
                }

                let pcm = capture.read_latest(FFT_SIZE);
                if !pcm.is_empty() {
                    analyzer.update_with_dt(&pcm, dt);
                }

                let frame = renderer.render(&analyzer, dt);

                // Update conflated frame slot for direct HID thread if running
                {
                    let (lock, cvar) = &*frame_producer;
                    if let Ok(mut slot) = lock.lock() {
                        *slot = Some(frame);
                        cvar.notify_one();
                    }
                }

                // Update shared state for GUI canvas and spectrum analyzer display
                if let Ok(mut st) = state_clone.lock() {
                    st.levels = analyzer.levels();
                    st.peaks = analyzer.peaks();
                    st.bass = analyzer.bass;
                    st.beat_detected = analyzer.beat_detected;
                    st.frame = frame;
                }

                let target_interval =
                    Duration::from_micros((1_000_000 / cur_fps.clamp(10, 90) as u64).max(11_111));
                let elapsed = start_tick.elapsed();
                if elapsed < target_interval {
                    thread::sleep(target_interval - elapsed);
                }
            }

            // Cleanup when stopped
            if let Ok(mut st) = state_clone.lock() {
                st.is_active = false;
                st.levels = [0.0; AUDIO_BANDS];
                st.peaks = [0.0; AUDIO_BANDS];
                st.bass = 0.0;
                st.beat_detected = false;
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
        if let Some(h) = self.hid_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for AudioVisualizerWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
