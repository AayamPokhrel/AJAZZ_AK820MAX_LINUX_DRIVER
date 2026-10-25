use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

pub const VID_BY_TECH: u16 = 0x258A;
pub const PID_BY_TECH_AK820: u16 = 0x010C;

pub const VID_COMPX_24G: u16 = 0x3554;
pub const PID_COMPX_24G_AK820: u16 = 0xFA09;

pub const VID_SHINETEK: u16 = 0x1A2C;
pub const PID_SHINETEK_AK820_WIRED: u16 = 0xA036;
pub const PID_SHINETEK_AK820_24G: u16 = 0x8FFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolVariant {
    /// 64-byte framing with AA 55 CC 33 magic prefix
    ShinetekEvision,
    /// 20-byte framing (Report ID 0x13 / 0x15) and 8-byte Feature report 0x06
    ByComboWireless,
    /// Feature report 0x05 (lighting) and 0x06 (520 bytes RGB/config)
    ByComboWired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub devnode: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_name: String,
    pub interface: Option<u32>,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub protocol: ProtocolVariant,
    pub is_wired: bool,
}

impl DiscoveredDevice {
    pub fn is_config_interface(&self) -> bool {
        match self.protocol {
            ProtocolVariant::ShinetekEvision => {
                self.usage_page == Some(0xFF02) && self.usage == Some(0x0002)
            }
            ProtocolVariant::ByComboWireless => {
                // Compx 2.4G receiver configuration interface is interface 1 (Usage page 0xFF02)
                self.interface == Some(1) || self.usage_page == Some(0xFF02)
            }
            ProtocolVariant::ByComboWired => {
                // BY Tech wired configuration interface is interface 1 (Usage page 0xFF00)
                self.interface == Some(1) || self.usage_page == Some(0xFF00)
            }
        }
    }
}

/// Parse first top-level collection usage page and usage from raw HID report descriptor
pub fn parse_first_usage(desc: &[u8]) -> (Option<u16>, Option<u16>) {
    let mut i = 0;
    let mut page = None;
    let mut usage = None;

    while i < desc.len() {
        let b = desc[i];
        if b == 0xFE {
            // Long item
            if i + 1 >= desc.len() {
                break;
            }
            i += 3 + desc[i + 1] as usize;
            continue;
        }

        let size = match b & 3 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };

        if i + 1 + size > desc.len() {
            break;
        }

        let tag = b & 0xFC;
        let val = match size {
            1 => desc[i + 1] as u32,
            2 => u16::from_le_bytes([desc[i + 1], desc[i + 2]]) as u32,
            4 => u32::from_le_bytes([desc[i + 1], desc[i + 2], desc[i + 3], desc[i + 4]]),
            _ => 0,
        };

        match tag {
            0x04 => page = Some(val as u16), // Usage Page (Global)
            0x08 if usage.is_none() => {
                if size == 4 {
                    page = Some((val >> 16) as u16);
                    usage = Some((val & 0xFFFF) as u16);
                } else {
                    usage = Some(val as u16);
                }
            }
            0xA0 => return (page, usage), // Collection reached
            _ => {}
        }

        i += 1 + size;
    }

    (page, usage)
}

/// Enumerate all connected Ajazz keyboards from `/sys/class/hidraw`
pub fn enumerate_devices() -> Vec<DiscoveredDevice> {
    let hidraw_dir = Path::new("/sys/class/hidraw");
    let mut devices = Vec::new();

    if !hidraw_dir.exists() {
        return devices;
    }

    let entries = match fs::read_dir(hidraw_dir) {
        Ok(e) => e,
        Err(_) => return devices,
    };

    for entry in entries.flatten() {
        let entry_name = entry.file_name().to_string_lossy().to_string();
        let devnode = format!("/dev/{}", entry_name);
        let dev_path = entry.path().join("device");

        let uevent_content = fs::read_to_string(dev_path.join("uevent")).unwrap_or_default();
        let mut vid = 0u16;
        let mut pid = 0u16;
        let mut hid_name = String::new();

        for line in uevent_content.lines() {
            if let Some(rest) = line.strip_prefix("HID_ID=") {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() >= 3 {
                    vid = u16::from_str_radix(parts[1], 16).unwrap_or(0);
                    pid = u16::from_str_radix(parts[2], 16).unwrap_or(0);
                }
            } else if let Some(rest) = line.strip_prefix("HID_NAME=") {
                hid_name = rest.to_string();
            }
        }

        let protocol = match (vid, pid) {
            (VID_BY_TECH, PID_BY_TECH_AK820) => ProtocolVariant::ByComboWired,
            (VID_COMPX_24G, PID_COMPX_24G_AK820) => ProtocolVariant::ByComboWireless,
            (VID_SHINETEK, PID_SHINETEK_AK820_WIRED) => ProtocolVariant::ShinetekEvision,
            (VID_SHINETEK, PID_SHINETEK_AK820_24G) => ProtocolVariant::ShinetekEvision,
            _ => continue, // Not an Ajazz AK820 device
        };

        let is_wired = (vid == VID_BY_TECH && pid == PID_BY_TECH_AK820)
            || (vid == VID_SHINETEK && pid == PID_SHINETEK_AK820_WIRED);

        // Read report descriptor
        let desc = fs::read(dev_path.join("report_descriptor")).unwrap_or_default();
        let (usage_page, usage) = parse_first_usage(&desc);

        // Determine interface number from sysfs symlink path (e.g. ".../3-1:1.1/...")
        let real_path = fs::canonicalize(entry.path()).unwrap_or_else(|_| PathBuf::from(&entry_name));
        let path_str = real_path.to_string_lossy();
        let interface = path_str
            .split(':')
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.parse::<u32>().ok());

        let dev = DiscoveredDevice {
            name: entry_name,
            devnode,
            vendor_id: vid,
            product_id: pid,
            device_name: hid_name,
            interface,
            usage_page,
            usage,
            protocol,
            is_wired,
        };

        trace!("Discovered candidate HID device: {:?}", dev);
        devices.push(dev);
    }

    // Filter to configuration interfaces and sort wired first
    let mut config_devs: Vec<DiscoveredDevice> = devices.into_iter().filter(|d| d.is_config_interface()).collect();
    config_devs.sort_by_key(|d| (!d.is_wired, d.name.clone()));

    debug!("Found [{}] Ajazz AK820 configuration interfaces", config_devs.len());
    config_devs
}
