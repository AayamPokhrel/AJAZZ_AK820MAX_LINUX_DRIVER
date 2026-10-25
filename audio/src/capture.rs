use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tracing::{debug, error, info};

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
}

pub struct AudioCapture {
    _stream: cpal::Stream,
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl AudioCapture {
    pub fn new(device_name: Option<&str>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        info!("Using CPAL audio host: {:?}", host.id());

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
            // Prefer monitor / loopback device on Linux (PipeWire/PulseAudio)
            let mut monitor_dev = None;
            if let Ok(devices) = host.input_devices() {
                for dev in devices {
                    if let Ok(name) = dev.name() {
                        let lower = name.to_lowercase();
                        if lower.contains("monitor") || lower.contains("loopback") {
                            debug!("Found audio monitor device: {}", name);
                            monitor_dev = Some(dev);
                            break;
                        }
                    }
                }
            }

            match monitor_dev {
                Some(dev) => dev,
                None => host
                    .default_input_device()
                    .ok_or(AudioError::NoDevice)?,
            }
        };

        let device_name = device.name().unwrap_or_else(|_| "unknown".to_string());
        info!("Capturing audio from device: {}", device_name);

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
            _stream: stream,
            sample_buffer,
            sample_rate,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Read the latest `count` samples from the capture buffer.
    pub fn read_latest(&self, count: usize) -> Vec<f32> {
        let buf = self.sample_buffer.lock().unwrap();
        if buf.len() <= count {
            buf.clone()
        } else {
            buf[buf.len() - count..].to_vec()
        }
    }
}
