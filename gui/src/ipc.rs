use ajazz_audio::visualizer::ColorMode;
use ajazz_hid::{enumerate_devices, AjazzDevice, DeviceStatus, DiscoveredDevice, HardwareConnection};
use ajazz_rgb::color::Color;
use ajazz_rgb::effects::{EffectMode, EffectSettings};
use ajazz_rgb::layout::TOTAL_LEDS;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
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
        color_mode: Option<String>,
        sensitivity: Option<f32>,
        noise_gate: Option<f32>,
        decay: Option<f32>,
        peak_hold: Option<bool>,
        source: Option<String>,
        fps: Option<u32>,
        morph_speed: Option<f32>,
    },
    StopMusic,
    ShuffleMusicZones,
    RerollMusicMosaic,
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

#[derive(Clone)]
pub struct CoordinatorSharedState {
    pub mode: ConnectionMode,
    pub status: Option<DeviceStatus>,
    pub device_info: Option<DiscoveredDevice>,
    pub available_devices: Vec<DiscoveredDevice>,
    pub preferred_connection: Option<HardwareConnection>,
    pub last_error: Option<String>,
    pub direct_device: Option<Arc<AjazzDevice>>,
}

enum CoordinatorCmd {
    Refresh { preferred: Option<HardwareConnection> },
    SetHardwareMode(Option<HardwareConnection>),
    DaemonReq(DaemonRequest),
    DirectSetLighting(EffectSettings),
    DirectSetCustomMatrix([Color; TOTAL_LEDS], u8),
    DirectSetSleep(u8),
    DirectEnterMusic,
}

/// Unified, non-blocking hardware manager for the GUI.
pub struct HardwareCoordinator {
    pub mode: ConnectionMode,
    pub status: Option<DeviceStatus>,
    pub device_info: Option<DiscoveredDevice>,
    pub available_devices: Vec<DiscoveredDevice>,
    pub preferred_connection: Option<HardwareConnection>,
    pub last_error: Option<String>,
    direct_device: Option<Arc<AjazzDevice>>,
    custom_matrix: [Color; TOTAL_LEDS],
    shared: Arc<parking_lot::RwLock<CoordinatorSharedState>>,
    cmd_tx: std::sync::mpsc::Sender<CoordinatorCmd>,
}

impl HardwareCoordinator {
    pub fn new() -> Self {
        // Enumerate local physical devices for immediate startup display
        let devs = enumerate_devices();
        let initial_info = devs.first().cloned();

        let initial_shared = CoordinatorSharedState {
            mode: ConnectionMode::Disconnected,
            status: None,
            device_info: initial_info.clone(),
            available_devices: devs.clone(),
            preferred_connection: None,
            last_error: None,
            direct_device: None,
        };

        let shared = Arc::new(parking_lot::RwLock::new(initial_shared));
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<CoordinatorCmd>();

        let shared_bg = Arc::clone(&shared);
        thread::Builder::new()
            .name("coord-bg".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(r) => r,
                    Err(_) => return,
                };

                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        CoordinatorCmd::Refresh { preferred } => {
                            let devs = enumerate_devices();
                            // 1. Try daemon first
                            let res = rt.block_on(send_daemon_request(DaemonRequest::GetStatus));
                            if let Ok(DaemonResponse::Status(st)) = res {
                                let mut s = shared_bg.write();
                                s.mode = ConnectionMode::ConnectedDaemon;
                                s.status = Some(st);
                                s.available_devices = devs.clone();
                                s.direct_device = None;
                                s.last_error = None;
                                if let Some(pref) = preferred {
                                    s.device_info = devs.into_iter().find(|d| d.connection_mode == pref);
                                } else {
                                    s.device_info = devs.into_iter().next();
                                }
                                continue;
                            }

                            // 2. Fallback to direct HID
                            match AjazzDevice::open_with_preference(preferred, None) {
                                Ok(dev) => {
                                    let st = dev.status().ok();
                                    let info = dev.info().clone();
                                    let mut s = shared_bg.write();
                                    s.mode = ConnectionMode::ConnectedDirectHid;
                                    s.status = st;
                                    s.device_info = Some(info);
                                    s.available_devices = devs;
                                    s.direct_device = Some(Arc::new(dev));
                                    s.last_error = None;
                                }
                                Err(e) => {
                                    let mut s = shared_bg.write();
                                    s.mode = ConnectionMode::Disconnected;
                                    s.status = None;
                                    s.device_info = None;
                                    s.available_devices = devs;
                                    s.direct_device = None;
                                    s.last_error = Some(format!("No AK820 Max device accessible: {}", e));
                                }
                            }
                        }
                        CoordinatorCmd::SetHardwareMode(pref) => {
                            let mode_str = match pref {
                                Some(HardwareConnection::Wired) => "wired",
                                Some(HardwareConnection::Wireless24G) => "2.4g",
                                Some(HardwareConnection::Bluetooth) => "bluetooth",
                                _ => "auto",
                            };
                            let _ = rt.block_on(send_daemon_request(DaemonRequest::SetMode {
                                mode: mode_str.to_string(),
                                device: None,
                            }));

                            // Re-probe status
                            let devs = enumerate_devices();
                            let res = rt.block_on(send_daemon_request(DaemonRequest::GetStatus));
                            if let Ok(DaemonResponse::Status(st)) = res {
                                let mut s = shared_bg.write();
                                s.mode = ConnectionMode::ConnectedDaemon;
                                s.status = Some(st);
                                s.available_devices = devs.clone();
                                s.direct_device = None;
                                s.last_error = None;
                                if let Some(p) = pref {
                                    s.device_info = devs.into_iter().find(|d| d.connection_mode == p);
                                } else {
                                    s.device_info = devs.into_iter().next();
                                }
                            } else {
                                match AjazzDevice::open_with_preference(pref, None) {
                                    Ok(dev) => {
                                        let st = dev.status().ok();
                                        let info = dev.info().clone();
                                        let mut s = shared_bg.write();
                                        s.mode = ConnectionMode::ConnectedDirectHid;
                                        s.status = st;
                                        s.device_info = Some(info);
                                        s.available_devices = devs;
                                        s.direct_device = Some(Arc::new(dev));
                                        s.last_error = None;
                                    }
                                    Err(e) => {
                                        let mut s = shared_bg.write();
                                        s.mode = ConnectionMode::Disconnected;
                                        s.status = None;
                                        s.device_info = None;
                                        s.available_devices = devs;
                                        s.direct_device = None;
                                        s.last_error = Some(format!("No AK820 Max device accessible: {}", e));
                                    }
                                }
                            }
                        }
                        CoordinatorCmd::DaemonReq(req) => {
                            let _ = rt.block_on(send_daemon_request(req));
                        }
                        CoordinatorCmd::DirectSetLighting(settings) => {
                            let dev_opt = shared_bg.read().direct_device.clone();
                            if let Some(dev) = dev_opt {
                                let _ = dev.set_lighting(&settings);
                            }
                        }
                        CoordinatorCmd::DirectSetCustomMatrix(matrix, brightness) => {
                            let dev_opt = shared_bg.read().direct_device.clone();
                            if let Some(dev) = dev_opt {
                                let _ = dev.set_custom_matrix(&matrix, brightness);
                            }
                        }
                        CoordinatorCmd::DirectSetSleep(mins) => {
                            let dev_opt = shared_bg.read().direct_device.clone();
                            if let Some(dev) = dev_opt {
                                let _ = dev.set_sleep_timeout(mins);
                            }
                        }
                        CoordinatorCmd::DirectEnterMusic => {
                            let dev_opt = shared_bg.read().direct_device.clone();
                            if let Some(dev) = dev_opt {
                                let _ = dev.enter_live_music_mode();
                            }
                        }
                    }
                }
            })
            .ok();

        // Trigger initial background refresh immediately
        let _ = cmd_tx.send(CoordinatorCmd::Refresh { preferred: None });

        Self {
            mode: ConnectionMode::Disconnected,
            status: None,
            device_info: initial_info,
            available_devices: devs,
            preferred_connection: None,
            last_error: None,
            direct_device: None,
            custom_matrix: [Color::BLACK; TOTAL_LEDS],
            shared,
            cmd_tx,
        }
    }

    /// Synchronize local GUI cached fields with background worker state without blocking
    pub fn sync(&mut self) {
        let s = self.shared.read();
        self.mode = s.mode.clone();
        self.status = s.status.clone();
        self.device_info = s.device_info.clone();
        self.available_devices = s.available_devices.clone();
        self.preferred_connection = s.preferred_connection;
        self.last_error = s.last_error.clone();
        self.direct_device = s.direct_device.clone();
    }

    pub fn direct_device(&self) -> Option<Arc<AjazzDevice>> {
        self.direct_device.as_ref().map(Arc::clone)
    }

    /// Set preferred hardware connection (Wired, 2.4G, Bluetooth, or None for Auto)
    pub fn set_hardware_mode(&mut self, pref: Option<HardwareConnection>) {
        self.preferred_connection = pref;
        let _ = self.cmd_tx.send(CoordinatorCmd::SetHardwareMode(pref));
        self.sync();
    }

    /// Refresh connection status asynchronously without stalling GUI render loop
    pub fn refresh(&mut self) {
        let _ = self.cmd_tx.send(CoordinatorCmd::Refresh {
            preferred: self.preferred_connection,
        });
        self.sync();
    }

    /// Set RGB lighting effect without blocking the GUI thread.
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
            let _ = self.cmd_tx.send(CoordinatorCmd::DaemonReq(req));
            Ok(())
        } else if self.direct_device.is_some() {
            let _ = self
                .cmd_tx
                .send(CoordinatorCmd::DirectSetLighting(settings.clone()));
            Ok(())
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
            let _ = self.cmd_tx.send(CoordinatorCmd::DaemonReq(req));
            Ok(())
        } else if self.direct_device.is_some() {
            let settings = EffectSettings {
                mode: EffectMode::Static,
                brightness: 5,
                speed: 4,
                direction: ajazz_rgb::effects::Direction::Right,
                palette_color: ajazz_rgb::color::PaletteColor::Rainbow,
                custom_color: Some(color),
            };
            let _ = self.cmd_tx.send(CoordinatorCmd::DirectSetLighting(settings));
            Ok(())
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
            let _ = self.cmd_tx.send(CoordinatorCmd::DaemonReq(req));
            Ok(())
        } else if self.direct_device.is_some() {
            let _ = self.cmd_tx.send(CoordinatorCmd::DirectSetCustomMatrix(
                self.custom_matrix,
                5,
            ));
            Ok(())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Apply an entire custom LED matrix without blocking.
    pub fn upload_custom_matrix(
        &mut self,
        matrix: &[Color; TOTAL_LEDS],
        brightness: u8,
    ) -> Result<(), String> {
        self.custom_matrix = *matrix;

        if self.mode == ConnectionMode::ConnectedDaemon {
            let colors: Vec<String> = matrix.iter().map(|c| c.to_hex()).collect();
            let req = DaemonRequest::SetCustomMatrix {
                colors,
                brightness: Some(brightness),
            };
            let _ = self.cmd_tx.send(CoordinatorCmd::DaemonReq(req));
            Ok(())
        } else if self.direct_device.is_some() {
            let _ = self
                .cmd_tx
                .send(CoordinatorCmd::DirectSetCustomMatrix(*matrix, brightness));
            Ok(())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Hot-reload or start music visualizer stream in background daemon (<1ms response).
    pub fn update_music_config(
        &mut self,
        style: &str,
        color_mode: &ColorMode,
        color: Option<Color>,
        palette: Option<&str>,
        sensitivity: f32,
        noise_gate: Option<f32>,
        decay: Option<f32>,
        peak_hold: Option<bool>,
        source: Option<&str>,
        fps: u32,
        morph_speed: Option<f32>,
    ) {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::StartMusic {
                style: style.to_string(),
                color: color.map(|c| c.to_hex()),
                palette: palette.map(|p| p.to_string()),
                color_mode: Some(color_mode.name().to_string()),
                sensitivity: Some(sensitivity),
                noise_gate,
                decay,
                peak_hold,
                source: source.map(|s| s.to_string()),
                fps: Some(fps),
                morph_speed,
            };
            let _ = self.cmd_tx.send(CoordinatorCmd::DaemonReq(req));
        }
    }

    /// Start music visualizer stream on hardware.
    pub fn start_music(
        &mut self,
        style: &str,
        color_mode: &ColorMode,
        color: Option<Color>,
        palette: Option<&str>,
        sensitivity: f32,
        noise_gate: Option<f32>,
        decay: Option<f32>,
        peak_hold: Option<bool>,
        source: Option<&str>,
        fps: u32,
        morph_speed: Option<f32>,
    ) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let req = DaemonRequest::StartMusic {
                style: style.to_string(),
                color: color.map(|c| c.to_hex()),
                palette: palette.map(|p| p.to_string()),
                color_mode: Some(color_mode.name().to_string()),
                sensitivity: Some(sensitivity),
                noise_gate,
                decay,
                peak_hold,
                source: source.map(|s| s.to_string()),
                fps: Some(fps),
                morph_speed,
            };
            let _ = self.cmd_tx.send(CoordinatorCmd::DaemonReq(req));
            Ok(())
        } else if self.direct_device.is_some() {
            let _ = self.cmd_tx.send(CoordinatorCmd::DirectEnterMusic);
            Ok(())
        } else {
            Err("No keyboard connection available".to_string())
        }
    }

    /// Shuffle zone gradients on daemon
    pub fn shuffle_music_zones(&mut self) {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let _ = self
                .cmd_tx
                .send(CoordinatorCmd::DaemonReq(DaemonRequest::ShuffleMusicZones));
        }
    }

    /// Re-roll random mosaic on daemon
    pub fn reroll_music_mosaic(&mut self) {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let _ = self
                .cmd_tx
                .send(CoordinatorCmd::DaemonReq(DaemonRequest::RerollMusicMosaic));
        }
    }

    /// Stop background music visualizer.
    pub fn stop_music(&mut self) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let _ = self
                .cmd_tx
                .send(CoordinatorCmd::DaemonReq(DaemonRequest::StopMusic));
        }
        Ok(())
    }

    /// Configure hardware sleep timeout in minutes (0 = never).
    pub fn set_sleep_timeout(&mut self, minutes: u8) -> Result<(), String> {
        if self.mode == ConnectionMode::ConnectedDaemon {
            let _ = self
                .cmd_tx
                .send(CoordinatorCmd::DaemonReq(DaemonRequest::SetSleep {
                    minutes,
                }));
            Ok(())
        } else if self.direct_device.is_some() {
            let _ = self.cmd_tx.send(CoordinatorCmd::DirectSetSleep(minutes));
            Ok(())
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

