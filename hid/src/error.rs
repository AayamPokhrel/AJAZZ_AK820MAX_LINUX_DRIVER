use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("device not found: {0}")]
    NotFound(String),
    #[error("permission denied opening {0}; install udev rules or check user groups")]
    PermissionDenied(String),
    #[error("I/O error on device {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("HID API error: {0}")]
    HidApi(#[from] hidapi::HidError),
    #[error("device disconnected or broken pipe")]
    Disconnected,
    #[error("protocol framing or timeout error: {0}")]
    Protocol(String),
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),
}
