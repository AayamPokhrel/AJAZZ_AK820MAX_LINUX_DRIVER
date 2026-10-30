use ajazz_hid::{enumerate_devices, AjazzDevice, DeviceStatus, DiscoveredDevice, HardwareConnection};
use ajazz_rgb::color::Color;
use ajazz_rgb::effects::{EffectMode, EffectSettings};
use ajazz_rgb::layout::TOTAL_LEDS;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime_dir).join("ajazz-ak820.sock");
        if let Some(parent) = p.parent() {
            if parent.exists() {
                return p;
            }
        }
    }
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/ajazz-ak820-{}.sock", uid))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum DaemonRequest {
    Ping,
    GetStatus,
    GetDevices,
    SetMode {
        mode: String,
        device: Option<String>,
    },
    SetEffect {
        mode: String,
        brightness: Option<u8>,
        speed: Option<u8>,
        direction: Option<String>,
        color: Option<String>,
    },
    SetStatic {
        color: String,
    },
    StartMusic {
        style: String,
        color: Option<String>,
        palette: Option<String>,
        sensitivity: Option<f32>,
        noise_gate: Option<f32>,
        source: Option<String>,
        fps: Option<u32>,
    },
    StopMusic,
    SetCustomKey {
        key: String,
        color: String,
    },
    SetCustomMatrix {
        colors: Vec<String>,
        brightness: Option<u8>,
    },
    SetSleep {
        minutes: u8,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum DaemonResponse {
    Ok { message: String },
    Pong,
    Status(DeviceStatus),
    Devices(Vec<DiscoveredDevice>),
    Error { message: String },
}

pub async fn send_daemon_request(req: DaemonRequest) -> Result<DaemonResponse, String> {
    let socket_path = default_socket_path();
    let stream = UnixStream::connect(&socket_path)
        .await
        .map_err(|e| format!("Could not connect to {}: {}", socket_path.display(), e))?;

    let (reader, mut writer) = stream.into_split();
    let mut payload = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    payload.push(b'\n');

    writer
        .write_all(&payload)
        .await
        .map_err(|e| format!("IPC write error: {}", e))?;

    let mut lines = BufReader::new(reader).lines();
    if let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        let resp: DaemonResponse = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        Ok(resp)
    } else {
        Err("Daemon closed connection without response".to_string())
    }
}

/// Connection mode for the GUI.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionMode {
    ConnectedDaemon,
    ConnectedDirectHid,
    Disconnected,
}

/// Unified hardware manager for the GUI.
pub struct HardwareCoordinator {
    pub mode: ConnectionMode,
    pub status: Option<DeviceStatus>,
    pub device_info: Option<DiscoveredDevice>,
    pub available_devices: Vec<DiscoveredDevice>,
    pub preferred_connection: Option<HardwareConnection>,
    pub last_error: Option<String>,
    direct_device: Option<Arc<AjazzDevice>>,
    custom_matrix: [Color; TOTAL_LEDS],
}

impl HardwareCoordinator {
    pub fn new() -> Self {
        Self {
            mode: ConnectionMode::Disconnected,
            status: None,
            device_info: None,
            available_devices: Vec::new(),
            preferred_connection: None,
            last_error: None,
            direct_device: None,
            custom_matrix: [Color::BLACK; TOTAL_LEDS],
        }
    }

    pub fn direct_device(&self) -> Option<Arc<AjazzDevice>> {
        self.direct_device.as_ref().map(Arc::clone)
    }

    /// Set preferred hardware connection (Wired, 2.4G, Bluetooth, or None for Auto)
    pub fn set_hardware_mode(&mut self, pref: Option<HardwareConnection>) {
        self.preferred_connection = pref;
        let mode_str = match pref {
            Some(HardwareConnection::Wired) => "wired",
            Some(HardwareConnection::Wireless24G) => "2.4g",
            Some(HardwareConnection::Bluetooth) => "bluetooth",
            _ => "auto",
        };

        // Notify daemon if running
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        if let Ok(rt) = rt {
            let _ = rt.block_on(async {
                send_daemon_request(DaemonRequest::SetMode {
                    mode: mode_str.to_string(),
                    device: None,
                })
                .await
            });
        }

        self.refresh();
    }

    /// Refresh connection status: try daemon first, fallback to direct HID.
    pub fn refresh(&mut self) {
        // Enumerate local physical devices for device_info
        let devs = enumerate_devices();
        self.available_devices = devs.clone();

        // 1. Try daemon IPC
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            let res = rt.block_on(async {
                send_daemon_request(DaemonRequest::GetStatus).await
            });

            if let Ok(DaemonResponse::Status(st)) = res {
                self.mode = ConnectionMode::ConnectedDaemon;
                self.status = Some(st);
                self.last_error = None;
                self.direct_device = None;

                // Set device_info matching preference if available
                if let Some(pref) = self.preferred_connection {
                    self.device_info = devs.into_iter().find(|d| d.connection_mode == pref);
                } else {
                    self.device_info = devs.into_iter().next();
                }
                return;
            }
        }

        // 2. Fallback to Direct HID
        match AjazzDevice::open_with_preference(self.preferred_connection, None) {
            Ok(dev) => {
                let st = dev.status().ok();
                self.device_info = Some(dev.info().clone());
                self.status = st;
                self.mode = ConnectionMode::ConnectedDirectHid;
                self.last_error = None;
                self.direct_device = Some(Arc::new(dev));
            }
            Err(e) => {
                self.mode = ConnectionMode::Disconnected;
                self.status = None;
                self.direct_device = None;
                self.device_info = None;
                self.last_error = Some(format!("No AK820 Max device accessible: {}", e));
            }
        }
    }

    /// Set RGB lighting effect.
    pub fn set_effect(&mut self, settings: &EffectSettings) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::SetEffect {
                mode: settings.mode.name().to_string(),
                brightness: Some(settings.brightness),
                speed: Some(settings.speed),
                direction: Some(if settings.direction == ajazz_rgb::effects::Direction::Right {
                    "right".to_string()
                } else {
                    "left".to_string()
                }),
                color: settings.custom_color.map(|c| c.to_hex()),
            };

            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else if let Some(dev) = &self.direct_device {
            dev.set_lighting(settings).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Set static color across all keys.
    #[allow(dead_code)]
    pub fn set_static(&mut self, color: Color) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::SetStatic {
                color: color.to_hex(),
            };
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else if let Some(dev) = &self.direct_device {
            let settings = EffectSettings {
                mode: EffectMode::Static,
                brightness: 5,
                speed: 4,
                direction: ajazz_rgb::effects::Direction::Right,
                palette_color: ajazz_rgb::color::PaletteColor::Rainbow,
                custom_color: Some(color),
            };
            dev.set_lighting(&settings).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Set an individual key color.
    #[allow(dead_code)]
    pub fn set_key_color(&mut self, led_index: usize, color: Color) -> Result<(), String> {
        if led_index >= TOTAL_LEDS {
            return Err("LED index out of range".to_string());
        }
        self.custom_matrix[led_index] = color;

        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::SetCustomKey {
                key: format!("idx:{}", led_index),
                color: color.to_hex(),
            };
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            let _ = rt.block_on(send_daemon_request(req));
            Ok(())
        } else if let Some(dev) = &self.direct_device {
            dev.set_custom_matrix(&self.custom_matrix, 5).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Apply an entire custom LED matrix.
    pub fn upload_custom_matrix(&mut self, matrix: &[Color; TOTAL_LEDS], brightness: u8) -> Result<(), String> {
        self.custom_matrix = *matrix;

        if self.mode == ConnectionMode::ConnectedDaemon {
            let colors: Vec<String> = matrix.iter().map(|c| c.to_hex()).collect();
            let req = DaemonRequest::SetCustomMatrix {
                colors,
                brightness: Some(brightness),
            };
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else if let Some(dev) = &self.direct_device {
            dev.set_custom_matrix(matrix, brightness).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Start music visualizer stream in background or prepare direct device.
    pub fn start_music_stream(
        &mut self,
        style: &str,
        color: Option<Color>,
        palette: Option<&str>,
        sensitivity: f32,
        noise_gate: Option<f32>,
        source: Option<&str>,
        fps: u32,
    ) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::StartMusic {
                style: style.to_string(),
                color: color.map(|c| c.to_hex()),
                palette: palette.map(|p| p.to_string()),
                sensitivity: Some(sensitivity),
                noise_gate,
                source: source.map(|s| s.to_string()),
                fps: Some(fps),
            };
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else if let Some(ref dev) = self.direct_device {
            dev.enter_live_music_mode().map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Alias for start_music_stream
    pub fn start_music(
        &mut self,
        style: &str,
        color: Option<Color>,
        palette: Option<&str>,
        sensitivity: f32,
        noise_gate: Option<f32>,
        source: Option<&str>,
        fps: u32,
    ) -> Result<(), String> {
        self.start_music_stream(style, color, palette, sensitivity, noise_gate, source, fps)
    }

    /// Stop background music visualizer.
    pub fn stop_music_stream(&mut self) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(DaemonRequest::StopMusic)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    /// Alias for stop_music_stream
    pub fn stop_music(&mut self) -> Result<(), String> {
        self.stop_music_stream()
    }

    /// Configure hardware sleep timeout in minutes (0 = never).
    pub fn set_sleep_timeout(&mut self, minutes: u8) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(DaemonRequest::SetSleep { minutes })) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else if let Some(dev) = &self.direct_device {
            dev.set_sleep_timeout(minutes).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Access local custom matrix cache.
    #[allow(dead_code)]
    pub fn custom_matrix(&self) -> &[Color; TOTAL_LEDS] {
        &self.custom_matrix
    }
}
