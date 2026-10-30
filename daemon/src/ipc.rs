use ajazz_hid::{DeviceStatus, DiscoveredDevice};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
