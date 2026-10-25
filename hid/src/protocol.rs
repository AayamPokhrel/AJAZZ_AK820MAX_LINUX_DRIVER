use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{EffectMode, EffectSettings};
use ajazz_rgb::layout::TOTAL_LEDS;
use serde::{Deserialize, Serialize};

pub const SHINETEK_PACKET_LEN: usize = 64;
pub const SHINETEK_MAGIC: &[u8; 4] = &[0xAA, 0x55, 0xCC, 0x33];

pub const CMD_KEY_ASSIGN: u8 = 0x04;
pub const CMD_SET_LIGHT: u8 = 0x07;
pub const CMD_RESET: u8 = 0x08;
pub const CMD_STREAM_BEGIN: u8 = 0x09;
pub const CMD_CUSTOM_PREPARE: u8 = 0x0B;
pub const CMD_SLEEP_TIME: u8 = 0x0C;
pub const CMD_RESPONSE_TIME: u8 = 0x0D;
pub const CMD_STATUS: u8 = 0x0E;

pub const CHUNK_DATA_LEN: usize = 54;
pub const CHUNK_COUNT: usize = 6;
pub const CUSTOM_CHUNK_BASE: u8 = 0xF6;
pub const LIVE_CHUNK_BASE: u8 = 0xF0;

pub const BYCOMBO_PACKET_LEN: usize = 20;
pub const BYCOMBO_REPORT_ID_OUT: u8 = 0x13;
pub const BYCOMBO_CMD_STREAM: u8 = 0x88;
pub const BYCOMBO_CMD_CUSTOM: u8 = 0x8E;
pub const BYCOMBO_CMD_SET_LED: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    Unknown,
    Charging,
    OnBattery,
    FullyCharged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub battery_percentage: Option<u8>,
    pub power_state: PowerState,
    pub is_charging: bool,
    pub raw: Vec<u8>,
}

// -----------------------------------------------------------------------------
// Shinetek Framing
// -----------------------------------------------------------------------------

pub fn build_shinetek_packet(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let mut pkt = vec![0u8; SHINETEK_PACKET_LEN];
    pkt[0..4].copy_from_slice(SHINETEK_MAGIC);
    pkt[4] = cmd;
    let len = payload.len().min(SHINETEK_PACKET_LEN - 5);
    pkt[5..5 + len].copy_from_slice(&payload[..len]);
    pkt
}

pub fn shinetek_light_packet(settings: &EffectSettings) -> Vec<u8> {
    let mode_byte = settings.mode.mode_id().saturating_sub(1);
    let bri_byte = settings.brightness.clamp(1, 5) - 1;
    let speed_byte = settings.speed.clamp(1, 5) - 1;
    let dir_byte = settings.direction as u8;
    let col_byte = settings.palette_color as u8;

    build_shinetek_packet(
        CMD_SET_LIGHT,
        &[mode_byte, bri_byte, speed_byte, dir_byte, col_byte],
    )
}

pub fn shinetek_status_request() -> Vec<u8> {
    build_shinetek_packet(CMD_STATUS, &[])
}

pub fn parse_shinetek_status(resp: &[u8]) -> Option<DeviceStatus> {
    if resp.len() < 7 || &resp[0..4] != SHINETEK_MAGIC || resp[4] != CMD_STATUS {
        return None;
    }

    let battery = if resp[5] <= 100 { Some(resp[5]) } else { None };
    let (power_state, is_charging) = match resp[6] {
        1 => (PowerState::Charging, true),
        2 => (PowerState::OnBattery, false),
        3 => (PowerState::FullyCharged, true),
        _ => (PowerState::Unknown, false),
    };

    Some(DeviceStatus {
        battery_percentage: battery,
        power_state,
        is_charging,
        raw: resp.to_vec(),
    })
}

pub fn shinetek_rgb_chunks(matrix: &[Color; TOTAL_LEDS], base_chunk: u8) -> Vec<Vec<u8>> {
    let mut buffer = [0u8; TOTAL_LEDS * 3];
    for (i, col) in matrix.iter().enumerate() {
        buffer[i * 3] = col.r;
        buffer[i * 3 + 1] = col.g;
        buffer[i * 3 + 2] = col.b;
    }

    let mut chunks = Vec::with_capacity(CHUNK_COUNT);
    for i in 0..CHUNK_COUNT {
        let chunk_cmd = base_chunk + i as u8;
        let start = i * CHUNK_DATA_LEN;
        let end = (start + CHUNK_DATA_LEN).min(buffer.len());
        chunks.push(build_shinetek_packet(chunk_cmd, &buffer[start..end]));
    }
    chunks
}

// -----------------------------------------------------------------------------
// BY-COMBO Framing (Wireless 2.4G & Wired)
// -----------------------------------------------------------------------------

pub fn build_bycombo_packet(report_id: u8, cmd: u8, seq: u8, payload: &[u8]) -> Vec<u8> {
    let mut pkt = vec![0u8; BYCOMBO_PACKET_LEN];
    pkt[0] = report_id;
    pkt[1] = 0x15;
    pkt[2] = cmd;
    pkt[3] = seq;

    let copy_len = payload.len().min(14);
    pkt[4..4 + copy_len].copy_from_slice(&payload[..copy_len]);

    // Checksum: modulo 256 sum of bytes 0..18
    let checksum: u8 = pkt[0..19].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    pkt[19] = checksum;
    pkt
}

pub fn bycombo_rgb_packets(matrix: &[Color; TOTAL_LEDS], cmd: u8) -> Vec<Vec<u8>> {
    let mut buffer = [0u8; TOTAL_LEDS * 3];
    for (i, col) in matrix.iter().enumerate() {
        buffer[i * 3] = col.r;
        buffer[i * 3 + 1] = col.g;
        buffer[i * 3 + 2] = col.b;
    }

    let payload_per_packet = 14;
    let packet_count = (buffer.len() + payload_per_packet - 1) / payload_per_packet;
    let mut packets = Vec::with_capacity(packet_count);

    for seq in 0..packet_count {
        let start = seq * payload_per_packet;
        let end = (start + payload_per_packet).min(buffer.len());
        packets.push(build_bycombo_packet(
            BYCOMBO_REPORT_ID_OUT,
            cmd,
            seq as u8,
            &buffer[start..end],
        ));
    }

    packets
}

pub fn parse_bycombo_status(feature_buf: &[u8]) -> Option<DeviceStatus> {
    if feature_buf.len() < 6 || feature_buf[0] != 0x06 {
        return None;
    }

    let battery = if feature_buf[3] <= 100 {
        Some(feature_buf[3])
    } else {
        None
    };

    let (power_state, is_charging) = match feature_buf[2] {
        1 => (PowerState::Charging, true),
        2 => (PowerState::OnBattery, false),
        3 => (PowerState::FullyCharged, true),
        _ => (PowerState::Unknown, false),
    };

    Some(DeviceStatus {
        battery_percentage: battery,
        power_state,
        is_charging,
        raw: feature_buf.to_vec(),
    })
}

/// Convert high-level EffectMode to BY-COMBO hardware mode ID
pub fn bycombo_hw_mode(mode: EffectMode) -> u8 {
    match mode {
        EffectMode::Static => 1,
        EffectMode::Spectrum => 2,
        EffectMode::Breathing => 3,
        EffectMode::KeyBreathing | EffectMode::Ejection => 19,
        EffectMode::Rain => 15,
        EffectMode::Scroll | EffectMode::Kaleidoscope => 13,
        EffectMode::RippleLr => 20,
        EffectMode::Starry => 16,
        EffectMode::TopBottom => 18,
        EffectMode::RippleUd => 5,
        EffectMode::Cross => 7,
        EffectMode::Galloping => 17,
        EffectMode::SineWave => 12,
        EffectMode::Windmill => 28,
        EffectMode::Music => 22,
        EffectMode::Screen => 23,
        EffectMode::Custom => 21,
        EffectMode::Off => 0,
    }
}

/// Convert high-level PaletteColor to BY-COMBO palette color index
pub fn bycombo_palette_color(color: PaletteColor) -> u8 {
    match color {
        PaletteColor::Rainbow => 0,
        PaletteColor::Red => 1,
        PaletteColor::Green => 2,
        PaletteColor::Blue => 3,
        PaletteColor::Yellow => 4,
        PaletteColor::Magenta => 5,
        PaletteColor::Cyan => 6,
        PaletteColor::White => 7,
    }
}

/// Build full Report ID 6 profile packet for BY-COMBO Wired mode
pub fn bycombo_wired_profile_packet(settings: &EffectSettings) -> Vec<u8> {
    let mut pkt = vec![0u8; 520];
    pkt[0] = 0x06; // Report ID 6
    pkt[1] = 0x03; // Command Write Profile
    pkt[2] = 0xB6; // Profile Subcommand
    pkt[12] = 0x01; // Enable
    let hw_mode = bycombo_hw_mode(settings.mode);
    pkt[13] = hw_mode;
    pkt[14] = 0x00; // Profile 0
    pkt[22] = 0xFF;
    pkt[23] = 0xFF;

    let col_byte = bycombo_palette_color(settings.palette_color);
    let bri_byte = (settings.brightness.clamp(1, 5) - 1) & 0x0F;
    let speed_byte = (settings.speed.clamp(1, 5) - 1) & 0x0F;
    let dir_byte = settings.direction as u8 & 1;

    for m in 0..20 {
        pkt[24 + m * 2] = (col_byte & 0x0F) | (dir_byte << 7);
        pkt[24 + m * 2 + 1] = (bri_byte << 4) | speed_byte;
    }
    pkt
}

/// Build 20-byte Report ID 0x13 packet for BY-COMBO Wireless 2.4GHz
pub fn bycombo_wireless_lighting_packet(settings: &EffectSettings) -> Vec<u8> {
    let hw_mode = bycombo_hw_mode(settings.mode);
    let bri_byte = settings.brightness.clamp(1, 5) - 1;
    let speed_byte = settings.speed.clamp(1, 5) - 1;
    let dir_byte = settings.direction as u8;
    let col_byte = bycombo_palette_color(settings.palette_color);

    build_bycombo_packet(
        BYCOMBO_REPORT_ID_OUT,
        BYCOMBO_CMD_SET_LED,
        0,
        &[hw_mode, bri_byte, speed_byte, dir_byte, col_byte],
    )
}

/// Build Report ID 6 feature packets for custom per-key matrix in BY-COMBO Wired mode
pub fn bycombo_wired_matrix_packets(matrix: &[Color; TOTAL_LEDS]) -> Vec<Vec<u8>> {
    let chunks = [(0xBCu8, 0usize, 42usize), (0xC0u8, 42usize, 42usize)];
    let mut out = Vec::new();

    for (chunk_id, start_idx, count) in chunks {
        let mut pkt = vec![0u8; 520];
        pkt[0] = 0x06;
        pkt[1] = 0x09; // CMD_SET_MATRIX
        pkt[2] = chunk_id;
        pkt[3] = 0x40;

        for i in 0..count {
            let key_idx = start_idx + i;
            if key_idx < matrix.len() {
                let offset = 4 + i * 3;
                if offset + 2 < pkt.len() {
                    pkt[offset] = matrix[key_idx].r;
                    pkt[offset + 1] = matrix[key_idx].g;
                    pkt[offset + 2] = matrix[key_idx].b;
                }
            }
        }
        out.push(pkt);
    }
    out
}
