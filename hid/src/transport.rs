use crate::error::DeviceError;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;
use tracing::{debug, trace};

pub trait Transport: Send + Sync {
    fn write_packet(&mut self, pkt: &[u8]) -> Result<(), DeviceError>;
    fn read_packet(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, DeviceError>;
    fn send_feature(&mut self, data: &[u8]) -> Result<(), DeviceError>;
    fn get_feature(&mut self, report_id: u8, buf: &mut [u8]) -> Result<(), DeviceError>;
}

// Linux ioctl numbers for hidraw feature reports
// _IOC(_IOC_READ|_IOC_WRITE, 'H', 0x06, size)
fn hidiocsfeature(size: usize) -> u64 {
    (3u64 << 30) | ((b'H' as u64) << 8) | 0x06 | ((size as u64) << 16)
}

// _IOC(_IOC_READ|_IOC_WRITE, 'H', 0x07, size)
fn hidiocgfeature(size: usize) -> u64 {
    (3u64 << 30) | ((b'H' as u64) << 8) | 0x07 | ((size as u64) << 16)
}

pub struct HidrawTransport {
    path: String,
    file: File,
}

impl HidrawTransport {
    pub fn open(path: &str) -> Result<Self, DeviceError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied => DeviceError::PermissionDenied(path.to_string()),
                std::io::ErrorKind::NotFound => DeviceError::NotFound(path.to_string()),
                _ => DeviceError::Io {
                    path: path.to_string(),
                    source: e,
                },
            })?;

        debug!("Opened hidraw device at {}", path);
        Ok(Self {
            path: path.to_string(),
            file,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

fn to_hex_dump(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}

impl Transport for HidrawTransport {
    fn write_packet(&mut self, pkt: &[u8]) -> Result<(), DeviceError> {
        use std::io::Write;
        trace!("hidraw write [{}] bytes to {}", pkt.len(), self.path);
        self.file.write_all(pkt).map_err(|e| {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                DeviceError::Disconnected
            } else {
                DeviceError::HardwareIo {
                    syscall: "write",
                    path: self.path.clone(),
                    report_id: pkt.first().copied().unwrap_or(0),
                    len: pkt.len(),
                    hex_dump: to_hex_dump(&pkt[..pkt.len().min(32)]),
                    source: e,
                }
            }
        })
    }

    fn read_packet(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, DeviceError> {
        use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
        use std::io::Read;

        let raw_fd = self.file.as_raw_fd();
        let pfd = PollFd::new(unsafe { std::os::fd::BorrowedFd::borrow_raw(raw_fd) }, PollFlags::POLLIN);
        let poll_timeout = PollTimeout::try_from(timeout).unwrap_or(PollTimeout::MAX);

        let ret = poll(&mut [pfd], poll_timeout).map_err(|e| DeviceError::Io {
            path: self.path.clone(),
            source: std::io::Error::from_raw_os_error(e as i32),
        })?;

        if ret <= 0 {
            return Ok(None);
        }

        let mut buf = vec![0u8; 512];
        match self.file.read(&mut buf) {
            Ok(0) => Err(DeviceError::Disconnected),
            Ok(n) => {
                buf.truncate(n);
                trace!("hidraw read [{}] bytes from {}", n, self.path);
                Ok(Some(buf))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(DeviceError::HardwareIo {
                syscall: "read",
                path: self.path.clone(),
                report_id: 0,
                len: 512,
                hex_dump: String::new(),
                source: e,
            }),
        }
    }

    fn send_feature(&mut self, data: &[u8]) -> Result<(), DeviceError> {
        let fd: RawFd = self.file.as_raw_fd();
        let req = hidiocsfeature(data.len());
        trace!("hidraw HIDIOCSFEATURE [{}] bytes", data.len());

        let res = unsafe { libc::ioctl(fd, req, data.as_ptr()) };
        if res < 0 {
            let err = std::io::Error::last_os_error();
            return Err(DeviceError::HardwareIo {
                syscall: "HIDIOCSFEATURE",
                path: self.path.clone(),
                report_id: data.first().copied().unwrap_or(0),
                len: data.len(),
                hex_dump: to_hex_dump(&data[..data.len().min(32)]),
                source: err,
            });
        }
        Ok(())
    }

    fn get_feature(&mut self, report_id: u8, buf: &mut [u8]) -> Result<(), DeviceError> {
        if buf.is_empty() {
            return Ok(());
        }
        buf[0] = report_id;
        let fd: RawFd = self.file.as_raw_fd();
        let req = hidiocgfeature(buf.len());
        trace!("hidraw HIDIOCGFEATURE ID 0x{:02X} [{}] bytes", report_id, buf.len());

        let res = unsafe { libc::ioctl(fd, req, buf.as_mut_ptr()) };
        if res < 0 {
            let err = std::io::Error::last_os_error();
            return Err(DeviceError::HardwareIo {
                syscall: "HIDIOCGFEATURE",
                path: self.path.clone(),
                report_id,
                len: buf.len(),
                hex_dump: to_hex_dump(&buf[..buf.len().min(32)]),
                source: err,
            });
        }
        Ok(())
    }
}

