use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub const FFT_SIZE: usize = 2048;
pub const AUDIO_BANDS: usize = 10;
pub const F_LO: f32 = 40.0;
pub const F_HI: f32 = 16000.0;
pub const PEAK_HOLD_SECS: f32 = 0.25;
pub const PEAK_FALL_SPEED: f32 = 1.6;

pub struct SpectrumAnalyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    hanning_window: Vec<f32>,
    bins: Vec<(usize, usize)>,
    tilt: Vec<f32>,
    levels: [f32; AUDIO_BANDS],
    peaks: [f32; AUDIO_BANDS],
    peak_timers: [f32; AUDIO_BANDS],
    peak_db: f32,
    sensitivity: f32,
    noise_gate: f32,
    pub energy: f32,
    pub bass: f32,
}

impl SpectrumAnalyzer {
    pub fn new(sample_rate: u32, sensitivity: f32) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        let hanning_window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos())
            })
            .collect();

        // Calculate logarithmic frequency band edges
        let mut edges = Vec::with_capacity(AUDIO_BANDS + 1);
        let ratio = (F_HI / F_LO).powf(1.0 / AUDIO_BANDS as f32);
        let mut curr = F_LO;
        for _ in 0..=AUDIO_BANDS {
            edges.push(curr);
            curr *= ratio;
        }

        let freq_step = sample_rate as f32 / FFT_SIZE as f32;
        let mut bins = Vec::with_capacity(AUDIO_BANDS);
        let mut tilt = Vec::with_capacity(AUDIO_BANDS);

        for i in 0..AUDIO_BANDS {
            let lo = edges[i];
            let hi = edges[i + 1];
            let bin_lo = (lo / freq_step).floor() as usize;
            let bin_hi = ((hi / freq_step).ceil() as usize).max(bin_lo + 1);
            bins.push((bin_lo.min(FFT_SIZE / 2), bin_hi.min(FFT_SIZE / 2 + 1)));

            let center_freq = (lo * hi).sqrt();
            // Pink noise compensation (human ears hear high frequencies quieter)
            tilt.push((center_freq / 1000.0).powf(0.5));
        }

        Self {
            fft,
            hanning_window,
            bins,
            tilt,
            levels: [0.0; AUDIO_BANDS],
            peaks: [0.0; AUDIO_BANDS],
            peak_timers: [0.0; AUDIO_BANDS],
            peak_db: -30.0,
            sensitivity: sensitivity.clamp(0.1, 5.0),
            noise_gate: 0.03,
            energy: 0.0,
            bass: 0.0,
        }
    }

    pub fn set_sensitivity(&mut self, s: f32) {
        self.sensitivity = s.clamp(0.1, 5.0);
    }

    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    pub fn set_noise_gate(&mut self, gate: f32) {
        self.noise_gate = gate.clamp(0.0, 0.4);
    }

    pub fn noise_gate(&self) -> f32 {
        self.noise_gate
    }

    /// Process input PCM samples and update spectrum levels (assumes default dt ~0.033).
    pub fn update(&mut self, pcm: &[f32]) -> [f32; AUDIO_BANDS] {
        self.update_with_dt(pcm, 0.033)
    }

    /// Process input PCM samples and update spectrum levels with physics-based delta time.
    pub fn update_with_dt(&mut self, pcm: &[f32], dt: f32) -> [f32; AUDIO_BANDS] {
        let dt = dt.clamp(0.005, 0.1);
        let mut input = vec![0.0f32; FFT_SIZE];
        if pcm.len() >= FFT_SIZE {
            let start = pcm.len() - FFT_SIZE;
            for (i, &s) in pcm[start..].iter().enumerate() {
                input[i] = s * self.hanning_window[i];
            }
        } else {
            let offset = FFT_SIZE - pcm.len();
            for (i, &s) in pcm.iter().enumerate() {
                input[offset + i] = s * self.hanning_window[offset + i];
            }
        }

        let mut output = self.fft.make_output_vec();
        if self.fft.process(&mut input, &mut output).is_err() {
            return self.levels;
        }

        let normalizer = (FFT_SIZE as f32) / 4.0;
        let mut mags = [0.0f32; AUDIO_BANDS];

        for (band_idx, &(bin_lo, bin_hi)) in self.bins.iter().enumerate() {
            let mut max_val = 0.0f32;
            for bin in bin_lo..bin_hi {
                if bin < output.len() {
                    let norm_sq = output[bin].norm_sqr();
                    let mag = norm_sq.sqrt() / normalizer;
                    if mag > max_val {
                        max_val = mag;
                    }
                }
            }
            mags[band_idx] = max_val * self.tilt[band_idx];
        }

        // Convert to dB scale with auto-gain tracking
        let mut max_db = -100.0f32;
        let mut dbs = [0.0f32; AUDIO_BANDS];
        for (i, &m) in mags.iter().enumerate() {
            let db = 20.0 * (m + 1e-9).log10();
            dbs[i] = db;
            if db > max_db {
                max_db = db;
            }
        }

        // Smooth auto-gain
        self.peak_db = max_db.max(self.peak_db - 0.15).max(-60.0);
        let dynamic_span = 40.0 / self.sensitivity;

        // Physics-based decay factor scaled by dt
        let decay_factor = (-2.8 * dt).exp().clamp(0.70, 0.95);

        for (i, &db) in dbs.iter().enumerate() {
            let norm = ((db - (self.peak_db - dynamic_span)) / dynamic_span).clamp(0.0, 1.0);
            let val = if max_db < -70.0 || norm < self.noise_gate {
                0.0
            } else {
                norm
            };

            // Fast attack, smooth-gravity exponential decay
            if val > self.levels[i] {
                // Instant punch on transients (attack)
                self.levels[i] = val;
            } else {
                // Smooth gravitational drop
                self.levels[i] = (self.levels[i] * decay_factor).max(val);
            }

            // Peak Hold ballistics
            if self.levels[i] >= self.peaks[i] {
                self.peaks[i] = self.levels[i];
                self.peak_timers[i] = PEAK_HOLD_SECS;
            } else if self.peak_timers[i] > 0.0 {
                self.peak_timers[i] -= dt;
            } else {
                // Peak drops with gravity
                self.peaks[i] = (self.peaks[i] - PEAK_FALL_SPEED * dt).max(self.levels[i]);
            }
        }

        self.energy = self.levels.iter().sum::<f32>() / AUDIO_BANDS as f32;
        let raw_bass = self.levels[0..3.min(AUDIO_BANDS)].iter().copied().fold(0.0f32, f32::max);
        self.bass = if raw_bass > self.bass {
            raw_bass
        } else {
            self.bass * decay_factor
        };

        self.levels
    }

    pub fn levels(&self) -> [f32; AUDIO_BANDS] {
        self.levels
    }

    pub fn peaks(&self) -> [f32; AUDIO_BANDS] {
        self.peaks
    }
}
