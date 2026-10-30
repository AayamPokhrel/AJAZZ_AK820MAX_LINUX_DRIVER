use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("no audio device available")]
    NoDevice,
    #[error("failed to get default stream config: {0}")]
    DefaultConfig(#[from] cpal::DefaultStreamConfigError),
    #[error("failed to build input stream: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error("failed to play input stream: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error("device error: {0}")]
    Device(#[from] cpal::DevicesError),
    #[error("PulseAudio/PipeWire error: {0}")]
    Pulse(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioSource {
    /// True Linux System Desktop Audio Output Loopback (@DEFAULT_MONITOR@)
    SystemLoopback,
    /// Default Physical Microphone Input
    Microphone,
    /// Named monitor or input device
    Custom(String),
}

impl Default for AudioSource {
    fn default() -> Self {
        Self::SystemLoopback
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSourceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub is_loopback: bool,
}

enum BackendHandle {
    Pulse {
        _running: Arc<AtomicBool>,
        _thread: Option<JoinHandle<()>>,
    },
    #[allow(dead_code)]
    Cpal(cpal::Stream),
}

pub struct AudioCapture {
    _handle: BackendHandle,
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    source_name: String,
}

impl AudioCapture {
    /// Create new audio capture. Defaults to System Desktop Audio Loopback.
    pub fn new(source: Option<AudioSource>) -> Result<Self, AudioError> {
        let src = source.unwrap_or(AudioSource::SystemLoopback);
        match src {
            AudioSource::SystemLoopback => {
                info!("Initializing System Desktop Audio Loopback (@DEFAULT_MONITOR@)...");
                match Self::init_pulse_loopback(Some("@DEFAULT_MONITOR@")) {
                    Ok(cap) => Ok(cap),
                    Err(e) => {
                        warn!("Could not connect to @DEFAULT_MONITOR@ ({}). Falling back to CPAL default input...", e);
                        Self::init_cpal(None)
                    }
                }
            }
            AudioSource::Microphone => {
                info!("Initializing Physical Microphone input via CPAL...");
                Self::init_cpal(None)
            }
            AudioSource::Custom(ref target) => {
                if target.eq_ignore_ascii_case("system") || target.eq_ignore_ascii_case("loopback") {
                    Self::init_pulse_loopback(Some("@DEFAULT_MONITOR@"))
                } else if target.eq_ignore_ascii_case("mic") || target.eq_ignore_ascii_case("microphone") {
                    Self::init_cpal(None)
                } else {
                    // Try pulse device name first, then CPAL device name
                    Self::init_pulse_loopback(Some(target)).or_else(|_| Self::init_cpal(Some(target)))
                }
            }
        }
    }

    fn init_pulse_loopback(device_name: Option<&str>) -> Result<Self, AudioError> {
        let sample_rate = 44100;
        let spec = Spec {
            format: Format::S16NE,
            channels: 1,
            rate: sample_rate,
        };

        if !spec.is_valid() {
            return Err(AudioError::Pulse("Invalid sample spec for PulseAudio".into()));
        }

        // Configure ultra-low-latency buffer attributes (~11.6ms fragment size, small max buffer)
        let buffer_attr = BufferAttr {
            maxlength: 2048 * 2, // ~46ms max buffer length
            tlength: u32::MAX,
            prebuf: u32::MAX,
            minreq: u32::MAX,
            fragsize: 512 * 2, // ~11.6ms low-latency fragment
        };

        let dev_str = device_name.unwrap_or("@DEFAULT_MONITOR@");
        let simple = Simple::new(
            None,
            "ajazz-ak820-visualizer",
            Direction::Record,
            Some(dev_str),
            "Desktop System Audio Loopback",
            &spec,
            None,
            Some(&buffer_attr),
        )
        .map_err(|e| AudioError::Pulse(format!("Simple::new error: {}", e)))?;

        let sample_buffer = Arc::new(Mutex::new(Vec::with_capacity(4096)));
        let buf_clone = Arc::clone(&sample_buffer);
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let thread_handle = thread::Builder::new()
            .name("ajazz-pulse-loopback".into())
            .spawn(move || {
                let mut chunk = [0i16; 512];
                let byte_len = chunk.len() * 2;
                while running_clone.load(Ordering::Relaxed) {
                    let slice = unsafe {
                        std::slice::from_raw_parts_mut(chunk.as_mut_ptr() as *mut u8, byte_len)
                    };
                    match simple.read(slice) {
                        Ok(()) => {
                            let mut buf = buf_clone.lock().unwrap();
                            for &s in &chunk {
                                buf.push(s as f32 / 32768.0);
                            }
                            // Ring buffer drain: keep strictly fresh window, avoid memory accumulation
                            if buf.len() > 4096 {
                                let drain_len = buf.len() - 2048;
                                buf.drain(0..drain_len);
                            }
                        }
                        Err(e) => {
                            error!("PulseAudio loopback read error: {}", e);
                            thread::sleep(std::time::Duration::from_millis(15));
                        }
                    }
                }
            })
            .map_err(|e| AudioError::Pulse(format!("Failed to spawn loopback thread: {}", e)))?;

        let dev_display = if dev_str == "@DEFAULT_MONITOR@" {
            "System Desktop Audio Output (Monitor Loopback)".to_string()
        } else {
            format!("System Monitor Loopback ({})", dev_str)
        };

        info!("Successfully active: {}", dev_display);

        Ok(Self {
            _handle: BackendHandle::Pulse {
                _running: running,
                _thread: Some(thread_handle),
            },
            sample_buffer,
            sample_rate,
            source_name: dev_display,
        })
    }

    fn init_cpal(device_name: Option<&str>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = if let Some(target) = device_name {
            let mut found = None;
            for dev in host.input_devices()? {
                if let Ok(name) = dev.name() {
                    if name.to_lowercase().contains(&target.to_lowercase()) {
                        found = Some(dev);
                        break;
                    }
                }
            }
            found.ok_or(AudioError::NoDevice)?
        } else {
            host.default_input_device().ok_or(AudioError::NoDevice)?
        };

        let device_name_str = device.name().unwrap_or_else(|_| "unknown".to_string());
        info!("CPAL capturing from device: {}", device_name_str);

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let sample_buffer = Arc::new(Mutex::new(Vec::with_capacity(16384)));
        let buf_clone = Arc::clone(&sample_buffer);

        let err_fn = |err| {
            error!("Audio stream error: {}", err);
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    let mut buf = buf_clone.lock().unwrap();
                    if channels > 1 {
                        for chunk in data.chunks_exact(channels) {
                            let mono = chunk.iter().sum::<f32>() / channels as f32;
                            buf.push(mono);
                        }
                    } else {
                        buf.extend_from_slice(data);
                    }
                    if buf.len() > 32768 {
                        let drain_len = buf.len() - 16384;
                        buf.drain(0..drain_len);
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    let mut buf = buf_clone.lock().unwrap();
                    if channels > 1 {
                        for chunk in data.chunks_exact(channels) {
                            let sum: f32 = chunk.iter().map(|&s| s as f32 / 32768.0).sum();
                            buf.push(sum / channels as f32);
                        }
                    } else {
                        buf.extend(data.iter().map(|&s| s as f32 / 32768.0));
                    }
                    if buf.len() > 32768 {
                        let drain_len = buf.len() - 16384;
                        buf.drain(0..drain_len);
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    let mut buf = buf_clone.lock().unwrap();
                    for &s in data {
                        buf.push((s as f32 - 32768.0) / 32768.0);
                    }
                    if buf.len() > 32768 {
                        let drain_len = buf.len() - 16384;
                        buf.drain(0..drain_len);
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(AudioError::NoDevice),
        };

        stream.play()?;

        Ok(Self {
            _handle: BackendHandle::Cpal(stream),
            sample_buffer,
            sample_rate,
            source_name: format!("Microphone ({})", device_name_str),
        })
    }

    /// List available audio sources for CLI and GUI selectors.
    pub fn available_sources() -> Vec<AudioSourceInfo> {
        let mut sources = vec![
            AudioSourceInfo {
                id: "system".into(),
                name: "System Desktop Audio (Output Loopback)".into(),
                is_default: true,
                is_loopback: true,
            },
            AudioSourceInfo {
                id: "mic".into(),
                name: "Microphone (Physical Input)".into(),
                is_default: false,
                is_loopback: false,
            },
        ];

        let host = cpal::default_host();
        if let Ok(devs) = host.input_devices() {
            for dev in devs {
                if let Ok(name) = dev.name() {
                    let is_loopback = name.to_lowercase().contains("monitor")
                        || name.to_lowercase().contains("loopback");
                    sources.push(AudioSourceInfo {
                        id: name.clone(),
                        name,
                        is_default: false,
                        is_loopback,
                    });
                }
            }
        }

        sources
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    /// Read the latest `count` samples from the capture buffer and drain older backlog.
    pub fn read_latest(&self, count: usize) -> Vec<f32> {
        let mut buf = self.sample_buffer.lock().unwrap();
        if buf.len() <= count {
            buf.clone()
        } else {
            let start = buf.len() - count;
            let res = buf[start..].to_vec();
            // Discard older queued audio chunks to ensure ZERO backlog
            buf.drain(0..start);
            res
        }
    }
}
