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
pub enum HardwareConnection {
    Wired,
    Wireless24G,
    Bluetooth,
    Unknown,
}

impl HardwareConnection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wired => "USB Wired",
            Self::Wireless24G => "2.4GHz Wireless",
            Self::Bluetooth => "Bluetooth",
            Self::Unknown => "Unknown",
        }
    }

    pub fn from_str_case(s: &str) -> Option<Self> {
        let cleaned = s.to_lowercase().replace(['-', '_', ' ', '.'], "");
        match cleaned.as_str() {
            "wired" | "usb" | "cable" => Some(Self::Wired),
            "wireless" | "wireless24g" | "24g" | "24ghz" | "dongle" | "rf" => Some(Self::Wireless24G),
            "bluetooth" | "bt" | "ble" => Some(Self::Bluetooth),
            _ => None,
        }
    }
}

pub type ConnectionMode = HardwareConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolVariant {
    /// 64-byte framing with AA 55 CC 33 magic prefix
    ShinetekEvision,
    /// 20-byte framing (Report ID 0x13 / 0x15) and 8-byte Feature report 0x06
    ByComboWireless,
    /// Feature report 0x05 (commands/status) and 0x06 (520 bytes profile/matrix)
    ByComboWired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub devnode: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub bus_type: u16,
    pub device_name: String,
    pub interface: Option<u32>,
    pub usage_pages: Vec<u16>,
    pub report_ids: Vec<u8>,
    pub protocol: ProtocolVariant,
    pub connection_mode: ConnectionMode,
    pub is_wired: bool,
}

impl DiscoveredDevice {
    pub fn is_config_interface(&self) -> bool {
        match self.protocol {
            ProtocolVariant::ShinetekEvision => {
                self.usage_pages.contains(&0xFF02)
                    || (self.interface == Some(1) && !self.report_ids.is_empty())
            }
            ProtocolVariant::ByComboWireless => {
                // Compx 2.4G receiver configuration interface has Report ID 0x13 / 0x06 and Page 0xFF02 / 0xFF04
                self.report_ids.contains(&0x13)
                    || self.usage_pages.contains(&0xFF02)
                    || self.interface == Some(1)
            }
            ProtocolVariant::ByComboWired => {
                // BY Tech wired configuration interface has Feature Reports 0x05 and 0x06 and Page 0xFF00
                (self.report_ids.contains(&0x05) && self.report_ids.contains(&0x06))
                    || self.usage_pages.contains(&0xFF00)
                    || self.interface == Some(1)
            }
        }
    }
}

/// Parse all usage pages and report IDs from a raw HID report descriptor
pub fn parse_descriptor_features(desc: &[u8]) -> (Vec<u16>, Vec<u8>) {
    let mut i = 0;
    let mut pages = Vec::new();
    let mut reports = Vec::new();

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
            0x04 => {
                let page = val as u16;
                if !pages.contains(&page) {
                    pages.push(page);
                }
            }
            0x84 => {
                let r_id = val as u8;
                if !reports.contains(&r_id) {
                    reports.push(r_id);
                }
            }
            _ => {}
        }

        i += 1 + size;
    }

    (pages, reports)
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
        let mut bus = 3u16; // default USB
        let mut vid = 0u16;
        let mut pid = 0u16;
        let mut hid_name = String::new();

        for line in uevent_content.lines() {
            if let Some(rest) = line.strip_prefix("HID_ID=") {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() >= 3 {
                    bus = u16::from_str_radix(parts[0], 16).unwrap_or(3);
                    vid = u16::from_str_radix(parts[1], 16).unwrap_or(0);
                    pid = u16::from_str_radix(parts[2], 16).unwrap_or(0);
                }
            } else if let Some(rest) = line.strip_prefix("HID_NAME=") {
                hid_name = rest.to_string();
            }
        }

        let name_lower = hid_name.to_lowercase();
        let is_known_id = (vid == VID_BY_TECH && pid == PID_BY_TECH_AK820)
            || (vid == VID_COMPX_24G && pid == PID_COMPX_24G_AK820)
            || (vid == VID_SHINETEK && (pid == PID_SHINETEK_AK820_WIRED || pid == PID_SHINETEK_AK820_24G));

        let is_name_matched = name_lower.contains("ak820")
            || name_lower.contains("ak820max")
            || (name_lower.contains("by tech") && name_lower.contains("keyboard"));

        if !is_known_id && !is_name_matched {
            continue;
        }

        // Determine Connection Mode
        let connection_mode = if bus == 5 || name_lower.contains("bluetooth") || name_lower.contains("bt") {
            ConnectionMode::Bluetooth
        } else if vid == VID_COMPX_24G
            || pid == PID_COMPX_24G_AK820
            || pid == PID_SHINETEK_AK820_24G
            || name_lower.contains("receiver")
            || name_lower.contains("2.4g")
            || name_lower.contains("dongle")
        {
            ConnectionMode::Wireless24G
        } else {
            ConnectionMode::Wired
        };

        // Determine protocol variant
        let protocol = match (vid, pid) {
            (VID_SHINETEK, _) => ProtocolVariant::ShinetekEvision,
            (VID_COMPX_24G, _) => ProtocolVariant::ByComboWireless,
            (VID_BY_TECH, _) => {
                if connection_mode == ConnectionMode::Wireless24G {
                    ProtocolVariant::ByComboWireless
                } else {
                    ProtocolVariant::ByComboWired
                }
            }
            _ => {
                if connection_mode == ConnectionMode::Wireless24G {
                    ProtocolVariant::ByComboWireless
                } else {
                    ProtocolVariant::ByComboWired
                }
            }
        };

        let is_wired = connection_mode == ConnectionMode::Wired;

        // Read report descriptor
        let desc = fs::read(dev_path.join("report_descriptor")).unwrap_or_default();
        let (usage_pages, report_ids) = parse_descriptor_features(&desc);

        // Determine interface number robustly from sysfs bInterfaceNumber or regex
        let iface_path = dev_path.join("../bInterfaceNumber");
        let interface = if let Ok(s) = fs::read_to_string(iface_path) {
            u32::from_str_radix(s.trim(), 16).ok().or_else(|| s.trim().parse::<u32>().ok())
        } else {
            let real_path = fs::canonicalize(entry.path()).unwrap_or_else(|_| PathBuf::from(&entry_name));
            let path_str = real_path.to_string_lossy();
            path_str
                .split(':')
                .last()
                .and_then(|s| s.split('/').next())
                .and_then(|s| s.split('.').last())
                .and_then(|s| s.parse::<u32>().ok())
        };

        let dev = DiscoveredDevice {
            name: entry_name,
            devnode,
            vendor_id: vid,
            product_id: pid,
            bus_type: bus,
            device_name: hid_name,
            interface,
            usage_pages,
            report_ids,
            protocol,
            connection_mode,
            is_wired,
        };

        trace!("Discovered candidate HID device: {:?}", dev);
        devices.push(dev);
    }

    // Filter to configuration interfaces
    let mut config_devs: Vec<DiscoveredDevice> = devices.into_iter().filter(|d| d.is_config_interface()).collect();

    // Sort order: Wired first, then 2.4G Wireless, then Bluetooth
    config_devs.sort_by_key(|d| {
        let mode_order = match d.connection_mode {
            ConnectionMode::Wired => 0,
            ConnectionMode::Wireless24G => 1,
            ConnectionMode::Bluetooth => 2,
            ConnectionMode::Unknown => 3,
        };
        (mode_order, d.name.clone())
    });

    debug!("Found [{}] Ajazz AK820 configuration interfaces", config_devs.len());
    config_devs
}

/// Find a device matching an optional mode or devnode path
pub fn find_device(
    preferred_mode: Option<ConnectionMode>,
    devnode_override: Option<&str>,
) -> Option<DiscoveredDevice> {
    let all = enumerate_devices();
    if all.is_empty() {
        return None;
    }

    if let Some(devnode) = devnode_override {
        if let Some(d) = all.iter().find(|d| d.devnode == devnode || d.name == devnode) {
            return Some(d.clone());
        }
    }

    if let Some(mode) = preferred_mode {
        if let Some(d) = all.iter().find(|d| d.connection_mode == mode) {
            return Some(d.clone());
        }
    }

    // Default: Return the top prioritized device (Wired over Wireless)
    all.into_iter().next()
}
