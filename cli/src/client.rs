use ajazz_hid::DeviceStatus;
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

pub async fn try_daemon_request(req: DaemonRequest) -> Result<DaemonResponse, String> {
    let socket_path = default_socket_path();
    if !socket_path.exists() {
        return Err("Daemon socket does not exist (daemon not running)".to_string());
    }

    let stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => return Err(format!("Could not connect to daemon: {}", e)),
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
