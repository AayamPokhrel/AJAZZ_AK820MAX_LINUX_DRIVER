use crate::discovery::{enumerate_devices, ConnectionMode, DiscoveredDevice, ProtocolVariant};
use crate::error::DeviceError;
use crate::protocol::*;
use crate::transport::{HidrawTransport, Transport};
use ajazz_rgb::color::Color;
use ajazz_rgb::effects::EffectSettings;
use ajazz_rgb::layout::TOTAL_LEDS;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct AjazzDevice {
    transport: Arc<Mutex<Box<dyn Transport>>>,
    device_info: DiscoveredDevice,
}

impl AjazzDevice {
    pub fn open_auto() -> Result<Self, DeviceError> {
        Self::open_with_preference(None, None)
    }

    pub fn open_with_preference(
        preferred_mode: Option<ConnectionMode>,
        devnode_override: Option<&str>,
    ) -> Result<Self, DeviceError> {
        let devs = enumerate_devices();
        if devs.is_empty() {
            return Err(DeviceError::NotFound(
                "No Ajazz AK820 Max device detected on USB Wired, 2.4G Wireless, or Bluetooth. Check connection or udev rules."
                    .to_string(),
            ));
        }

        // If explicit devnode was requested, find it
        if let Some(path) = devnode_override {
            let selected = devs
                .iter()
                .find(|d| d.devnode == path || d.name == path)
                .ok_or_else(|| {
                    DeviceError::NotFound(format!("Device path {} not matched as an Ajazz keyboard", path))
                })?
                .clone();

            info!(
                "Opening specified Ajazz device: {} ({}, mode: {:?})",
                selected.device_name, selected.devnode, selected.connection_mode
            );
            let transport = Box::new(HidrawTransport::open(&selected.devnode)?);
            return Ok(Self {
                transport: Arc::new(Mutex::new(transport)),
                device_info: selected,
            });
        }

        // If specific mode was preferred (e.g. Wired, Wireless24G, or Bluetooth)
        if let Some(mode) = preferred_mode {
            if let Some(selected) = devs.iter().find(|d| d.connection_mode == mode) {
                info!(
                    "Connecting to requested {:?} mode on {} ({})",
                    mode, selected.device_name, selected.devnode
                );
                let transport = Box::new(HidrawTransport::open(&selected.devnode)?);
                return Ok(Self {
                    transport: Arc::new(Mutex::new(transport)),
                    device_info: selected.clone(),
                });
            } else {
                warn!("Preferred mode {:?} not currently detected; checking available modes", mode);
            }
        }

        // Auto-Probe:
        // Try Wired candidates first since physical cable is prioritized over RF dongle
        for candidate in &devs {
            if candidate.connection_mode == ConnectionMode::Wired {
                if let Ok(mut t) = HidrawTransport::open(&candidate.devnode) {
                    // Test ping handshake via Report ID 5 GetPassword request
                    let ping = [0x05, 0x81, 0x00, 0x00, 0x00, 0x00];
                    if t.send_feature(&ping).is_ok() {
                        info!(
                            "Auto-detected active USB Wired keyboard on {}",
                            candidate.devnode
                        );
                        return Ok(Self {
                            transport: Arc::new(Mutex::new(Box::new(t))),
                            device_info: candidate.clone(),
                        });
                    }
                }
            }
        }

        // Next try Wireless / Dongle candidates
        for candidate in &devs {
            if candidate.connection_mode == ConnectionMode::Wireless24G {
                if let Ok(mut t) = HidrawTransport::open(&candidate.devnode) {
                    let mut buf = [0u8; 8];
                    if t.get_feature(0x06, &mut buf).is_ok() {
                        info!(
                            "Auto-detected active 2.4GHz Wireless receiver on {}",
                            candidate.devnode
                        );
                        return Ok(Self {
                            transport: Arc::new(Mutex::new(Box::new(t))),
                            device_info: candidate.clone(),
                        });
                    }
                }
            }
        }

        // Fall back to top prioritized candidate in list
        let selected = devs.into_iter().next().unwrap();
        info!(
            "Connecting to Ajazz keyboard: {} ({}, mode: {:?})",
            selected.device_name, selected.devnode, selected.connection_mode
        );

        let transport = Box::new(HidrawTransport::open(&selected.devnode)?);
        Ok(Self {
            transport: Arc::new(Mutex::new(transport)),
            device_info: selected,
        })
    }

    pub fn open_path(path: &str) -> Result<Self, DeviceError> {
        Self::open_with_preference(None, Some(path))
    }

    pub fn info(&self) -> &DiscoveredDevice {
        &self.device_info
    }

    pub fn protocol(&self) -> ProtocolVariant {
        self.device_info.protocol
    }

    pub fn connection_mode(&self) -> ConnectionMode {
        self.device_info.connection_mode
    }

    pub fn status(&self) -> Result<DeviceStatus, DeviceError> {
        let mut t = self.transport.lock().unwrap();
        match self.device_info.protocol {
            ProtocolVariant::ShinetekEvision => {
                let req = shinetek_status_request();
                t.write_packet(&req)?;
                std::thread::sleep(Duration::from_millis(50));
                let resp = t.read_packet(Duration::from_millis(300))?;
                match resp {
                    Some(bytes) => parse_shinetek_status(&bytes)
                        .ok_or_else(|| DeviceError::Protocol("invalid Shinetek status payload".into())),
                    None => Err(DeviceError::Protocol("timeout waiting for device status".into())),
                }
            }
            ProtocolVariant::ByComboWireless => {
                let mut buf = [0u8; 8];
                t.get_feature(0x06, &mut buf)?;
                parse_bycombo_status(&buf)
                    .ok_or_else(|| DeviceError::Protocol("invalid BY-COMBO wireless status".into()))
            }
            ProtocolVariant::ByComboWired => {
                // Query current LED buffer via Report ID 6 to test hardware responsiveness
                let req = bycombo_wired_get_led_request();
                t.send_feature(&req)?;
                std::thread::sleep(Duration::from_millis(20));
                let mut buf = vec![0u8; 520];
                t.get_feature(0x06, &mut buf)?;

                Ok(DeviceStatus {
                    battery_percentage: Some(100),
                    power_state: PowerState::FullyCharged,
                    is_charging: true,
                    raw: buf,
                })
            }
        }
    }

    pub fn set_lighting(&self, settings: &EffectSettings) -> Result<(), DeviceError> {
        let mut t = self.transport.lock().unwrap();
        match self.device_info.protocol {
            ProtocolVariant::ShinetekEvision => {
                let pkt = shinetek_light_packet(settings);
                t.write_packet(&pkt)?;
                std::thread::sleep(Duration::from_millis(15));
            }
            ProtocolVariant::ByComboWireless => {
                let pkt = bycombo_wireless_lighting_packet(settings);
                t.write_packet(&pkt)?;
                std::thread::sleep(Duration::from_millis(15));
            }
            ProtocolVariant::ByComboWired => {
                // 1. Fetch current 128-byte LED block
                let get_req = bycombo_wired_get_led_request();
                let mut current_buf = vec![0u8; 520];
                if t.send_feature(&get_req).is_ok() {
                    std::thread::sleep(Duration::from_millis(20));
                    let _ = t.get_feature(0x06, &mut current_buf);
                }

                // 2. Build 1:1 vendor SetLED packet (520 bytes)
                let payload_slice = if current_buf.len() >= 136 && current_buf[1] == 0x84 {
                    Some(&current_buf[8..8 + 128])
                } else {
                    None
                };
                let pkt = bycombo_wired_set_led_packet(payload_slice, settings);
                t.send_feature(&pkt)?;
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        debug!("Applied lighting effect: {:?}", settings.mode);
        Ok(())
    }

    pub fn enter_live_music_mode(&self) -> Result<(), DeviceError> {
        let mut t = self.transport.lock().unwrap();
        match self.device_info.protocol {
            ProtocolVariant::ShinetekEvision => {
                let pkt = build_shinetek_packet(CMD_SET_LIGHT, &[21, 4, 2, 0, 7]);
                t.write_packet(&pkt)?;
                std::thread::sleep(Duration::from_millis(20));
                let begin = build_shinetek_packet(CMD_STREAM_BEGIN, &[]);
                t.write_packet(&begin)?;
                std::thread::sleep(Duration::from_millis(30));
            }
            ProtocolVariant::ByComboWireless => {
                let init = build_bycombo_packet(BYCOMBO_REPORT_ID_OUT, BYCOMBO_CMD_STREAM, 0, &[1]);
                t.write_packet(&init)?;
                std::thread::sleep(Duration::from_millis(20));
            }
            ProtocolVariant::ByComboWired => {
                // Switch to Custom / Matrix mode (hw 21)
                let get_req = bycombo_wired_get_led_request();
                let mut current_buf = vec![0u8; 520];
                if t.send_feature(&get_req).is_ok() {
                    std::thread::sleep(Duration::from_millis(20));
                    let _ = t.get_feature(0x06, &mut current_buf);
                }
                let payload_slice = if current_buf.len() >= 136 && current_buf[1] == 0x84 {
                    Some(&current_buf[8..8 + 128])
                } else {
                    None
                };
                let settings = EffectSettings {
                    mode: ajazz_rgb::effects::EffectMode::Custom,
                    brightness: 5,
                    speed: 4,
                    direction: ajazz_rgb::effects::Direction::Right,
                    palette_color: ajazz_rgb::color::PaletteColor::Rainbow,
                    custom_color: None,
                };
                let pkt = bycombo_wired_set_led_packet(payload_slice, &settings);
                t.send_feature(&pkt)?;
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        info!("Device initialized into live music streaming mode");
        Ok(())
    }

    pub fn send_live_frame(&self, frame: &[Color; TOTAL_LEDS]) -> Result<(), DeviceError> {
        let mut t = self.transport.lock().unwrap();
        match self.device_info.protocol {
            ProtocolVariant::ShinetekEvision => {
                let chunks = shinetek_rgb_chunks(frame, LIVE_CHUNK_BASE);
                for chunk in chunks {
                    t.write_packet(&chunk)?;
                }
            }
            ProtocolVariant::ByComboWireless => {
                let packets = bycombo_rgb_packets(frame, BYCOMBO_CMD_STREAM);
                for pkt in packets {
                    t.write_packet(&pkt)?;
                }
            }
            ProtocolVariant::ByComboWired => {
                let pkt = bycombo_wired_matrix_packet(frame);
                t.send_feature(&pkt)?;
            }
        }
        Ok(())
    }

    pub fn set_custom_matrix(&self, matrix: &[Color; TOTAL_LEDS], brightness: u8) -> Result<(), DeviceError> {
        let mut t = self.transport.lock().unwrap();
        match self.device_info.protocol {
            ProtocolVariant::ShinetekEvision => {
                let mode_pkt = build_shinetek_packet(CMD_SET_LIGHT, &[23, brightness.clamp(1, 5) - 1, 0, 0, 0]);
                t.write_packet(&mode_pkt)?;
                std::thread::sleep(Duration::from_millis(20));

                let prep = build_shinetek_packet(CMD_CUSTOM_PREPARE, &[]);
                t.write_packet(&prep)?;
                std::thread::sleep(Duration::from_millis(20));

                let chunks = shinetek_rgb_chunks(matrix, CUSTOM_CHUNK_BASE);
                for chunk in chunks {
                    t.write_packet(&chunk)?;
                    std::thread::sleep(Duration::from_millis(3));
                }
            }
            ProtocolVariant::ByComboWireless => {
                let packets = bycombo_rgb_packets(matrix, BYCOMBO_CMD_CUSTOM);
                for pkt in packets {
                    t.write_packet(&pkt)?;
                    std::thread::sleep(Duration::from_millis(3));
                }
            }
            ProtocolVariant::ByComboWired => {
                // Switch to custom mode
                let get_req = bycombo_wired_get_led_request();
                let mut current_buf = vec![0u8; 520];
                if t.send_feature(&get_req).is_ok() {
                    std::thread::sleep(Duration::from_millis(20));
                    let _ = t.get_feature(0x06, &mut current_buf);
                }
                let payload_slice = if current_buf.len() >= 136 && current_buf[1] == 0x84 {
                    Some(&current_buf[8..8 + 128])
                } else {
                    None
                };
                let settings = EffectSettings {
                    mode: ajazz_rgb::effects::EffectMode::Custom,
                    brightness,
                    speed: 4,
                    direction: ajazz_rgb::effects::Direction::Right,
                    palette_color: ajazz_rgb::color::PaletteColor::Rainbow,
                    custom_color: None,
                };
                let mode_pkt = bycombo_wired_set_led_packet(payload_slice, &settings);
                t.send_feature(&mode_pkt)?;
                std::thread::sleep(Duration::from_millis(20));

                let matrix_pkt = bycombo_wired_matrix_packet(matrix);
                t.send_feature(&matrix_pkt)?;
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        info!("Per-key custom RGB matrix uploaded to keyboard");
        Ok(())
    }

    pub fn set_sleep_timeout(&self, minutes: u8) -> Result<(), DeviceError> {
        let idx = match minutes {
            0 => 3,       // Never
            1..=5 => 0,   // 5min
            6..=15 => 1,  // 15min
            _ => 2,       // 30min
        };

        let mut t = self.transport.lock().unwrap();
        match self.device_info.protocol {
            ProtocolVariant::ShinetekEvision => {
                let pkt = build_shinetek_packet(CMD_SLEEP_TIME, &[idx]);
                t.write_packet(&pkt)?;
            }
            ProtocolVariant::ByComboWireless => {
                let payload = [0x0C, idx];
                let pkt = build_bycombo_packet(BYCOMBO_REPORT_ID_OUT, 0x0C, 0, &payload);
                t.write_packet(&pkt)?;
            }
            ProtocolVariant::ByComboWired => {
                let get_req = bycombo_wired_get_cfg_request();
                let mut current_buf = vec![0u8; 520];
                if t.send_feature(&get_req).is_ok() {
                    std::thread::sleep(Duration::from_millis(20));
                    let _ = t.get_feature(0x06, &mut current_buf);
                }
                let existing_cfg = if current_buf.len() >= 28 && current_buf[1] == 0x83 {
                    Some(&current_buf[8..28])
                } else {
                    None
                };
                let set_pkt = bycombo_wired_set_cfg_packet(existing_cfg, minutes);
                t.send_feature(&set_pkt)?;
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        info!("Set keyboard sleep timeout to {} min (index {})", minutes, idx);
        Ok(())
    }
}

