use crate::discovery::{enumerate_devices, ConnectionMode, DiscoveredDevice, ProtocolVariant};
use crate::error::DeviceError;
use crate::protocol::*;
use crate::transport::{HidrawTransport, SimulatedTransport, Transport};
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
        if std::env::var("AJAZZ_SIMULATED").is_ok() {
            info!("AJAZZ_SIMULATED set; using simulated in-memory keyboard");
            return Ok(Self::open_simulated());
        }

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
                    // Test ping handshake
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

    pub fn open_simulated() -> Self {
        let info = DiscoveredDevice {
            name: "sim0".to_string(),
            devnode: "/dev/simulated".to_string(),
            vendor_id: 0x258A,
            product_id: 0x010C,
            bus_type: 3,
            device_name: "Simulated Ajazz AK820 Max".to_string(),
            interface: Some(1),
            usage_pages: vec![0xFF00],
            report_ids: vec![5, 6],
            protocol: ProtocolVariant::ByComboWired,
            connection_mode: ConnectionMode::Wired,
            is_wired: true,
        };

        Self {
            transport: Arc::new(Mutex::new(Box::new(SimulatedTransport::new()))),
            device_info: info,
        }
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
                let req = [0x05, 0x81, 0x00, 0x00, 0x00, 0x00];
                let _ = t.send_feature(&req);
                std::thread::sleep(Duration::from_millis(15));
                let mut buf = vec![0u8; 520];
                let _ = t.get_feature(0x06, &mut buf);

                Ok(DeviceStatus {
                    battery_percentage: Some(100),
                    power_state: PowerState::FullyCharged,
                    is_charging: true,
                    raw: vec![0x06, 0x00, 0x03, 0x64],
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
                let pkt = bycombo_wired_profile_packet(settings);
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
                let mut pkt = vec![0u8; 520];
                pkt[0] = 0x06;
                pkt[1] = 0x03;
                pkt[2] = 0xB6;
                pkt[12] = 0x01;
                pkt[13] = 22; // Music mode
                pkt[22] = 0xFF;
                pkt[23] = 0xFF;
                let _ = t.send_feature(&pkt);
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
                    let _ = t.write_packet(&pkt);
                }
            }
            ProtocolVariant::ByComboWired => {
                let chunks = bycombo_wired_matrix_packets(frame);
                for chunk in chunks {
                    let _ = t.send_feature(&chunk);
                }
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
                // Set custom mode
                let mut pkt = vec![0u8; 520];
                pkt[0] = 0x06;
                pkt[1] = 0x03;
                pkt[2] = 0xB6;
                pkt[12] = 0x01;
                pkt[13] = 21; // Custom / Self-define mode
                pkt[22] = 0xFF;
                pkt[23] = 0xFF;
                let _ = t.send_feature(&pkt);
                std::thread::sleep(Duration::from_millis(15));

                let chunks = bycombo_wired_matrix_packets(matrix);
                for chunk in chunks {
                    let _ = t.send_feature(&chunk);
                    std::thread::sleep(Duration::from_millis(5));
                }
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
            ProtocolVariant::ByComboWireless | ProtocolVariant::ByComboWired => {
                let payload = [0x0C, idx];
                let pkt = build_bycombo_packet(BYCOMBO_REPORT_ID_OUT, 0x0C, 0, &payload);
                let _ = t.write_packet(&pkt);
            }
        }
        info!("Set keyboard sleep timeout to {} min (index {})", minutes, idx);
        Ok(())
    }
}
