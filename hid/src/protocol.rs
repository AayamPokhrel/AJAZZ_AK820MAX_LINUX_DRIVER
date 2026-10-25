use ajazz_rgb::color::Color;
use ajazz_rgb::effects::EffectSettings;
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

pub fn bycombo_wired_light_feature(settings: &EffectSettings) -> [u8; 6] {
    let mode_byte = settings.mode.mode_id();
    let bri_byte = settings.brightness.clamp(1, 5);
    let speed_byte = settings.speed.clamp(1, 5);
    let dir_byte = settings.direction as u8;
    let col_byte = settings.palette_color as u8;

    [0x05, mode_byte, bri_byte, speed_byte, dir_byte, col_byte]
}

pub fn bycombo_wired_matrix_feature(matrix: &[Color; TOTAL_LEDS]) -> Vec<u8> {
    let mut buf = vec![0u8; 520];
    buf[0] = 0x06; // Report ID 6
    for (i, col) in matrix.iter().enumerate() {
        let offset = 1 + i * 3;
        if offset + 2 < buf.len() {
            buf[offset] = col.r;
            buf[offset + 1] = col.g;
            buf[offset + 2] = col.b;
        }
    }
    buf
}
