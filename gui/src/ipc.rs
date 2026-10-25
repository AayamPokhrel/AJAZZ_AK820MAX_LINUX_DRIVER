use ajazz_hid::{enumerate_devices, AjazzDevice, DeviceStatus, DiscoveredDevice};
use ajazz_rgb::color::Color;
use ajazz_rgb::effects::{EffectMode, EffectSettings};
use ajazz_rgb::layout::TOTAL_LEDS;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
        sensitivity: Option<f32>,
        fps: Option<u32>,
    },
    StopMusic,
    SetCustomKey {
        key: String,
        color: String,
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
    Error { message: String },
}

pub async fn send_daemon_request(req: DaemonRequest) -> Result<DaemonResponse, String> {
    let socket_path = default_socket_path();
    if !socket_path.exists() {
        return Err("Daemon socket does not exist (ak820d daemon is stopped)".to_string());
    }

    let stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => return Err(format!("Could not connect to ak820d daemon: {}", e)),
    };

    let (reader, mut writer) = stream.into_split();
    let mut data = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    data.push(b'\n');

    writer.write_all(&data).await.map_err(|e| e.to_string())?;

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
    pub last_error: Option<String>,
    direct_device: Option<AjazzDevice>,
    custom_matrix: [Color; TOTAL_LEDS],
}

impl HardwareCoordinator {
    pub fn new() -> Self {
        Self {
            mode: ConnectionMode::Disconnected,
            status: None,
            device_info: None,
            last_error: None,
            direct_device: None,
            custom_matrix: [Color::BLACK; TOTAL_LEDS],
        }
    }

    /// Refresh connection status: try daemon first, fallback to direct HID.
    pub fn refresh(&mut self) {
        // Enumerate local physical devices for device_info
        let devs = enumerate_devices();
        self.device_info = devs.into_iter().next();

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
                return;
            }
        }

        // 2. Fallback to Direct HID
        match AjazzDevice::open_auto() {
            Ok(dev) => {
                let st = dev.status().ok();
                self.device_info = Some(dev.info().clone());
                self.status = st;
                self.mode = ConnectionMode::ConnectedDirectHid;
                self.last_error = None;
                self.direct_device = Some(dev);
            }
            Err(e) => {
                self.mode = ConnectionMode::Disconnected;
                self.status = None;
                self.direct_device = None;
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
                custom_color: Some(color),
                brightness: 5,
                speed: 3,
                direction: ajazz_rgb::effects::Direction::Right,
                palette_color: ajazz_rgb::color::PaletteColor::Rainbow,
            };
            dev.set_lighting(&settings).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Set custom per-key color.
    #[allow(dead_code)]
    pub fn set_custom_key(&mut self, key_name: &str, color: Color) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::SetCustomKey {
                key: key_name.to_string(),
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
            if let Some(key) = ajazz_rgb::layout::find_key_by_name(key_name) {
                if key.led_idx < TOTAL_LEDS {
                    self.custom_matrix[key.led_idx] = color;
                    dev.set_custom_matrix(&self.custom_matrix, 5).map_err(|e| e.to_string())
                } else {
                    Ok(())
                }
            } else {
                Err(format!("Unknown key name: {}", key_name))
            }
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Upload entire custom painted matrix.
    pub fn upload_custom_matrix(&mut self, matrix: &[Color; TOTAL_LEDS]) -> Result<(), String> {
        self.custom_matrix = *matrix;
        if self.mode == ConnectionMode::ConnectedDaemon {
            let mut errs = 0;
            for key in ajazz_rgb::layout::KEYS {
                let col = matrix[key.led_idx];
                let req = DaemonRequest::SetCustomKey {
                    key: key.name.to_string(),
                    color: col.to_hex(),
                };
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
                if rt.block_on(send_daemon_request(req)).is_err() {
                    errs += 1;
                }
            }
            if errs == 0 {
                Ok(())
            } else {
                Err(format!("Completed with {} non-critical key errors", errs))
            }
        } else if let Some(dev) = &self.direct_device {
            dev.set_custom_matrix(matrix, 5).map_err(|e| e.to_string())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Start music visualizer stream.
    pub fn start_music(&mut self, style: &str, color: Option<Color>, sensitivity: f32, fps: u32) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::StartMusic {
                style: style.to_string(),
                color: color.map(|c| c.to_hex()),
                sensitivity: Some(sensitivity),
                fps: Some(fps),
            };
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else if let Some(dev) = &self.direct_device {
            dev.enter_live_music_mode().map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }

    /// Stop music visualizer stream.
    pub fn stop_music(&mut self) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::StopMusic;
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
                Ok(DaemonResponse::Ok { .. }) => Ok(()),
                Ok(DaemonResponse::Error { message }) => Err(message),
                Err(e) => Err(e),
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    /// Set auto-sleep timer in minutes (0 = never).
    pub fn set_sleep(&mut self, minutes: u8) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::SetSleep { minutes };
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            match rt.block_on(send_daemon_request(req)) {
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
}
