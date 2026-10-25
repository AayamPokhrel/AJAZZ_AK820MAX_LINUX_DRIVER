use ajazz_hid::{DeviceStatus, DiscoveredDevice};
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
    Devices(Vec<DiscoveredDevice>),
    Error { message: String },
}

pub async fn try_daemon_request(req: DaemonRequest) -> anyhow::Result<DaemonResponse> {
    let path = default_socket_path();
    let stream = UnixStream::connect(&path).await?;
    let (reader, mut writer) = stream.into_split();

    let mut line = serde_json::to_string(&req)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;

    let mut reader = BufReader::new(reader);
    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).await?;

    let resp: DaemonResponse = serde_json::from_str(&resp_line)?;
    Ok(resp)
}
