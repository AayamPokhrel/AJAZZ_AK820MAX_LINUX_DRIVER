use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub const FFT_SIZE: usize = 2048;
pub const AUDIO_BANDS: usize = 16;
pub const PEAK_HOLD_SECS: f32 = 0.28;
pub const PEAK_FALL_SPEED: f32 = 1.8;

/// Calibrated 16-band center frequency labels (Hz)
pub const BAND_LABELS: [&str; AUDIO_BANDS] = [
    "30", "60", "110", "180", "280", "450", "700", "1.1k",
    "1.8k", "2.8k", "4.2k", "6k", "8.5k", "11.5k", "14k", "18k",
];

pub struct SpectrumAnalyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    hanning_window: Vec<f32>,
    bins: Vec<(usize, usize)>,
    tilt: Vec<f32>,
    levels: [f32; AUDIO_BANDS],
    peaks: [f32; AUDIO_BANDS],
    peak_timers: [f32; AUDIO_BANDS],
    peak_hold_enabled: bool,
    band_peak_db: [f32; AUDIO_BANDS],
    global_peak_db: f32,
    sensitivity: f32,
    noise_gate: f32,
    decay_speed: f32,
    pub energy: f32,
    pub bass: f32,
    pub beat_detected: bool,
    last_bass: f32,
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

        // 16 Logarithmic ISO bands spanning 25 Hz to 20,000 Hz
        let band_edges_hz: [f32; AUDIO_BANDS + 1] = [
            25.0, 50.0, 85.0, 140.0, 230.0, 360.0, 580.0, 900.0,
            1400.0, 2200.0, 3500.0, 5200.0, 7500.0, 10500.0, 13500.0, 16500.0, 20000.0,
        ];

        let freq_step = sample_rate as f32 / FFT_SIZE as f32;
        let mut bins = Vec::with_capacity(AUDIO_BANDS);
        let mut tilt = Vec::with_capacity(AUDIO_BANDS);

        // Calibrated psychoacoustic equal-loudness curve to compensate for music spectral tilt:
        // Raw audio naturally has steep -3 to -6 dB/octave slope (pink noise).
        // This tilt table brings mid-range instruments, vocals, and high cymbals into equal balance with heavy sub-bass.
        let tilts: [f32; AUDIO_BANDS] = [
            0.65, // 25-50 Hz (sub-bass attenuation to prevent 0dB clamping)
            0.85, // 50-85 Hz (punch bass)
            1.00, // 85-140 Hz (bass body)
            1.15, // 140-230 Hz (warmth)
            1.30, // 230-360 Hz (low mids)
            1.55, // 360-580 Hz (mids / guitar / snare fundamental)
            1.80, // 580-900 Hz (vocal body)
            2.10, // 900-1400 Hz (vocal presence)
            2.45, // 1.4-2.2 kHz (ear canal resonance / presence)
            2.90, // 2.2-3.5 kHz (clarity / bite)
            3.40, // 3.5-5.2 kHz (upper presence)
            4.00, // 5.2-7.5 kHz (snare snap / sibilance)
            4.80, // 7.5-10.5 kHz (cymbal brilliance)
            5.70, // 10.5-13.5 kHz (air)
            6.80, // 13.5-16.5 kHz (high air)
            8.00, // 16.5-20.0 kHz (sparkle)
        ];

        for i in 0..AUDIO_BANDS {
            let lo = band_edges_hz[i];
            let hi = band_edges_hz[i + 1];
            let bin_lo = (lo / freq_step).floor() as usize;
            let bin_hi = ((hi / freq_step).ceil() as usize).max(bin_lo + 1);
            bins.push((bin_lo.min(FFT_SIZE / 2), bin_hi.min(FFT_SIZE / 2 + 1)));
            tilt.push(tilts[i]);
        }

        Self {
            fft,
            hanning_window,
            bins,
            tilt,
            levels: [0.0; AUDIO_BANDS],
            peaks: [0.0; AUDIO_BANDS],
            peak_timers: [0.0; AUDIO_BANDS],
            peak_hold_enabled: true,
            band_peak_db: [-40.0; AUDIO_BANDS],
            global_peak_db: -35.0,
            sensitivity: sensitivity.clamp(0.1, 5.0),
            noise_gate: 0.03,
            decay_speed: 2.8,
            energy: 0.0,
            bass: 0.0,
            beat_detected: false,
            last_bass: 0.0,
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

    pub fn set_decay_speed(&mut self, speed: f32) {
        self.decay_speed = speed.clamp(1.0, 6.0);
    }

    pub fn set_peak_hold(&mut self, enabled: bool) {
        self.peak_hold_enabled = enabled;
        if !enabled {
            self.peaks = [0.0; AUDIO_BANDS];
        }
    }

    pub fn peak_hold(&self) -> bool {
        self.peak_hold_enabled
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

        // Compute RMS energy across bins in each band to eliminate noise spikes
        for (band_idx, &(bin_lo, bin_hi)) in self.bins.iter().enumerate() {
            let mut sum_sq = 0.0f32;
            let mut count = 0;
            for bin in bin_lo..bin_hi {
                if bin < output.len() {
                    let norm_sq = output[bin].norm_sqr();
                    sum_sq += norm_sq;
                    count += 1;
                }
            }
            let rms = if count > 0 {
                (sum_sq / count as f32).sqrt() / normalizer
            } else {
                0.0
            };
            mags[band_idx] = rms * self.tilt[band_idx];
        }

        // Convert to decibels
        let mut max_db = -100.0f32;
        let mut dbs = [0.0f32; AUDIO_BANDS];
        for (i, &m) in mags.iter().enumerate() {
            let db = 20.0 * (m + 1e-9).log10();
            dbs[i] = db;
            if db > max_db {
                max_db = db;
            }
        }

        // Track global audio ceiling with fast attack & gradual release
        if max_db > self.global_peak_db {
            self.global_peak_db = max_db;
        } else {
            // Decay ceiling by ~8 dB per second
            self.global_peak_db = (self.global_peak_db - 8.0 * dt).max(-60.0);
        }

        let dynamic_span = 38.0 / self.sensitivity;

        // Exponential decay factor scaled by dt and decay_speed
        let decay_factor = (-self.decay_speed * dt).exp().clamp(0.65, 0.96);

        for i in 0..AUDIO_BANDS {
            let db = dbs[i];

            // Per-band AGC: track per-band peak with gentle decay so mid/highs stay lively even with loud sub-bass
            if db > self.band_peak_db[i] {
                self.band_peak_db[i] = db;
            } else {
                self.band_peak_db[i] = (self.band_peak_db[i] - 12.0 * dt).max(self.global_peak_db - dynamic_span);
            }

            // Balanced target floor using both global and local band dynamics
            let floor_db = self.global_peak_db.max(self.band_peak_db[i] + 6.0) - dynamic_span;
            let norm = ((db - floor_db) / dynamic_span).clamp(0.0, 1.0);

            let val = if max_db < -68.0 || norm < self.noise_gate {
                0.0
            } else {
                norm
            };

            // Instant punch on transients (attack), smooth gravitational decay
            if val > self.levels[i] {
                self.levels[i] = val;
            } else {
                self.levels[i] = (self.levels[i] * decay_factor).max(val);
            }

            // Peak Hold ballistics
            if self.peak_hold_enabled {
                if self.levels[i] >= self.peaks[i] {
                    self.peaks[i] = self.levels[i];
                    self.peak_timers[i] = PEAK_HOLD_SECS;
                } else if self.peak_timers[i] > 0.0 {
                    self.peak_timers[i] -= dt;
                } else {
                    self.peaks[i] = (self.peaks[i] - PEAK_FALL_SPEED * dt).max(self.levels[i]);
                }
            } else {
                self.peaks[i] = 0.0;
            }
        }

        self.energy = self.levels.iter().sum::<f32>() / AUDIO_BANDS as f32;
        let raw_bass = self.levels[0..3].iter().copied().fold(0.0f32, f32::max);

        // Beat detection: rapid transient surge in sub/bass bands (>0.15 step increase)
        self.beat_detected = raw_bass > 0.45 && (raw_bass - self.last_bass) > 0.12;
        self.last_bass = raw_bass;

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
