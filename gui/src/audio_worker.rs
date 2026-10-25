use ajazz_audio::capture::AudioCapture;
use ajazz_audio::spectrum::SpectrumAnalyzer;
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
    pub error: Option<String>,
}

impl Default for AudioVisualizerState {
    fn default() -> Self {
        Self {
            is_active: false,
            levels: [0.0; 10],
            peaks: [0.0; 10],
            bass: 0.0,
            error: None,
        }
    }
}

pub struct AudioVisualizerWorker {
    state: Arc<Mutex<AudioVisualizerState>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl AudioVisualizerWorker {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(AudioVisualizerState::default()));
        let running = Arc::new(AtomicBool::new(false));

        Self {
            state,
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

    pub fn start(&mut self, sensitivity: f32) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        let state_clone = Arc::clone(&self.state);
        let running_clone = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            info!("Starting GUI Audio Capture Worker thread...");
            let capture_res = AudioCapture::new(None);
            let capture = match capture_res {
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

            let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), sensitivity);
            let mut peaks = [0.0f32; 10];
            let mut last_instant = Instant::now();

            {
                let mut st = state_clone.lock().unwrap();
                st.is_active = true;
                st.error = None;
            }

            while running_clone.load(Ordering::SeqCst) {
                let start_tick = Instant::now();
                let dt = last_instant.elapsed().as_secs_f32().max(0.001);
                last_instant = Instant::now();

                // Read audio samples from ring buffer
                let samples = capture.read_latest(2048);
                if !samples.is_empty() {
                    analyzer.update(&samples);
                }

                let raw_levels = analyzer.levels();
                let mut current_levels = [0.0f32; 10];

                for i in 0..10 {
                    let scaled = (raw_levels[i] * sensitivity).clamp(0.0, 1.0);
                    current_levels[i] = scaled;

                    // Peak decay animation
                    if scaled >= peaks[i] {
                        peaks[i] = scaled;
                    } else {
                        peaks[i] = (peaks[i] - dt * 0.8).max(0.0);
                    }
                }

                // Update shared visualizer state
                if let Ok(mut st) = state_clone.lock() {
                    st.levels = current_levels;
                    st.peaks = peaks;
                    st.bass = (analyzer.bass * sensitivity).clamp(0.0, 1.0);
                }

                // Sleep to maintain ~60 FPS
                let elapsed = start_tick.elapsed();
                let target_interval = Duration::from_micros(16_666);
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
            }
            info!("GUI Audio Capture Worker stopped.");
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
