use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{Direction, EffectMode, EffectSettings};
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

/// Convert high-level EffectMode to BY-COMBO hardware mode ID matching KB.ini
pub fn bycombo_hw_mode(mode: EffectMode) -> u8 {
    match mode {
        EffectMode::Static => 1,
        EffectMode::Breathing => 2,
        EffectMode::SineWave => 3,
        EffectMode::Spectrum => 4,
        EffectMode::RippleLr => 5,
        EffectMode::RippleUd => 6,
        EffectMode::Starry => 7,
        EffectMode::Ejection => 8,
        EffectMode::Kaleidoscope => 9,
        EffectMode::Cross => 10,
        EffectMode::KeyBreathing => 11,
        EffectMode::Scroll => 12,
        EffectMode::TopBottom => 13,
        EffectMode::Rain => 14,
        EffectMode::Windmill => 15,
        EffectMode::Galloping => 16,
        EffectMode::Music => 17,
        EffectMode::Screen => 18,
        EffectMode::Custom => 21,
        EffectMode::Off => 0,
    }
}

/// Convert high-level PaletteColor to BY-COMBO palette color index
pub fn bycombo_palette_color(color: PaletteColor) -> u8 {
    match color {
        PaletteColor::Red => 0,
        PaletteColor::Yellow => 2,
        PaletteColor::Green => 3,
        PaletteColor::Cyan => 4,
        PaletteColor::Blue => 5,
        PaletteColor::Magenta => 6,
        PaletteColor::Rainbow | PaletteColor::White => 7,
    }
}

/// Request current 128-byte LED configuration from device via Report ID 6
pub fn bycombo_wired_get_led_request() -> Vec<u8> {
    let mut req = vec![0u8; 520];
    req[0] = 0x06;
    req[1] = 0x84; // GetLED request
    req[4] = 0x01; // 1 chunk
    req[6] = 0x80; // 128 bytes
    req
}

/// Build full Report ID 6 SetLED packet (520 bytes) for BY-COMBO Wired mode
/// 1:1 vendor implementation from CDevG5KB::AccessData & CDevG5KB::SetLED
pub fn bycombo_wired_set_led_packet(current_led_128: Option<&[u8]>, settings: &EffectSettings) -> Vec<u8> {
    let mut pkt = vec![0u8; 520];
    // 8-byte CDevG5KB::AccessData header:
    pkt[0] = 0x06; // Report ID 6
    pkt[1] = 0x04; // cmd = 4 (SetLED)
    pkt[2] = 0x01; // nOper = 1 (Write)
    pkt[3] = 0x00;
    pkt[4] = 0x01; // total_chunks = 1
    pkt[5] = 0x00; // chunk_index = 0
    pkt[6] = 0x80; // len lo = 128
    pkt[7] = 0x00; // len hi = 0

    // Initialize payload with existing state or defaults
    if let Some(existing) = current_led_128 {
        let copy_len = existing.len().min(128);
        pkt[8..8 + copy_len].copy_from_slice(&existing[..copy_len]);
    }

    let hw_id = bycombo_hw_mode(settings.mode);
    pkt[8 + 9] = 0x00;
    pkt[8 + 10] = hw_id;

    let bri = (settings.brightness.clamp(1, 5) - 1) & 0x0F;
    let spd = (settings.speed.clamp(1, 5) - 1) & 0x0F;
    let dir = if settings.direction == Direction::Left { 0x80 } else { 0x00 };
    let col = match settings.palette_color {
        PaletteColor::Red => 0,
        PaletteColor::Yellow => 2,
        PaletteColor::Green => 3,
        PaletteColor::Cyan => 4,
        PaletteColor::Blue => 5,
        PaletteColor::Magenta => 6,
        PaletteColor::Rainbow | PaletteColor::White => 7,
    };

    pkt[8 + 1] = spd;
    pkt[8 + 2] = bri;

    if hw_id >= 1 && hw_id <= 20 {
        let off = 8 + 0x3a + ((hw_id - 1) as usize) * 2;
        if off + 1 < pkt.len() {
            pkt[off] = dir | bri;
            pkt[off + 1] = (spd << 4) | (col & 0x0F);
        }
    }

    // Magic trailer verified from hardware and decompilation
    pkt[8 + 126] = 0x5A;
    pkt[8 + 127] = 0xA5;

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

/// Build full Report ID 6 live music frame packet (520 bytes) for BY-COMBO Wired mode
/// 1:1 vendor implementation from CDevG5KB::SendMusicData (CMD 0x08, len 0x017A / 378 bytes)
pub fn bycombo_wired_live_music_packet(matrix: &[Color; TOTAL_LEDS]) -> Vec<u8> {
    let mut pkt = vec![0u8; 520];
    pkt[0] = 0x06; // Report ID 6
    pkt[1] = 0x08; // cmd = 0x08 (SendMusicData)
    pkt[2] = 0x01; // nOper = 1 (Write)
    pkt[3] = 0x00;
    pkt[4] = 0x01; // total_chunks = 1
    pkt[5] = 0x00; // chunk_index = 0
    pkt[6] = 0x7A; // len lo = 122 (0x7A)
    pkt[7] = 0x01; // len hi = 1 -> 378 bytes (0x017A)

    // Populate keys directly by hardware LED matrix index
    for (i, c) in matrix.iter().enumerate().take(126) {
        let off = 8 + i * 3;
        pkt[off] = c.r;
        pkt[off + 1] = c.g;
        pkt[off + 2] = c.b;
    }

    pkt
}

/// Request matrix RGB dump via Report ID 6 GetLedRgbTab
pub fn bycombo_wired_get_matrix_request() -> Vec<u8> {
    let mut req = vec![0u8; 520];
    req[0] = 0x06;
    req[1] = 0x8A; // GetLedRgbTab (0x80 | 0x0A)
    req[4] = 0x01; // 1 chunk
    req[6] = 0x00; // 512 bytes = 0x0200
    req[7] = 0x02;
    req
}

/// Build full Report ID 6 SetLedRgbTab packet (520 bytes) for BY-COMBO Wired mode
/// 1:1 vendor implementation from CDevG5KB::AccessData & CDevG5KB::SetLedRgbTab
pub fn bycombo_wired_matrix_packet(matrix: &[Color; TOTAL_LEDS]) -> Vec<u8> {
    let mut pkt = vec![0u8; 520];
    pkt[0] = 0x06; // Report ID 6
    pkt[1] = 0x0A; // cmd = 0x0A (SetLedRgbTab)
    pkt[2] = 0x01; // nOper = 1 (Write)
    pkt[3] = 0x00;
    pkt[4] = 0x01; // total_chunks = 1
    pkt[5] = 0x00; // chunk_index = 0
    pkt[6] = 0x00; // len lo = 512 (0x0200)
    pkt[7] = 0x02; // len hi = 2

    // Populate keys directly by hardware LED matrix index
    for (i, c) in matrix.iter().enumerate().take(126) {
        let off = 8 + i * 3;
        if off + 2 < 8 + 510 {
            pkt[off] = c.r;
            pkt[off + 1] = c.g;
            pkt[off + 2] = c.b;
        }
    }

    // CRC / Magic trailer at end of 512-byte payload
    pkt[8 + 510] = 0x5A;
    pkt[8 + 511] = 0xA5;

    pkt
}


/// Request device configuration dump via Report ID 6 GetCfg (0x83)
pub fn bycombo_wired_get_cfg_request() -> Vec<u8> {
    let mut req = vec![0u8; 520];
    req[0] = 0x06;
    req[1] = 0x83; // GetCfg (0x80 | 0x03)
    req[2] = 0x02; // Read
    req[4] = 0x01; // 1 chunk
    req[6] = 0x14; // 20 bytes
    req
}

/// Build full Report ID 6 SetCfg packet (520 bytes) for BY-COMBO Wired mode
/// 1:1 vendor implementation from CDevG5KB::AccessData & CDevG5KB::SetCfg
pub fn bycombo_wired_set_cfg_packet(current_cfg: Option<&[u8]>, sleep_minutes: u8) -> Vec<u8> {
    let mut pkt = vec![0u8; 520];
    pkt[0] = 0x06; // Report ID 6
    pkt[1] = 0x03; // cmd = 0x03 (SetCfg)
    pkt[2] = 0x01; // nOper = 1 (Write)
    pkt[3] = 0x00;
    pkt[4] = 0x01; // total_chunks = 1
    pkt[5] = 0x00; // chunk_index = 0
    pkt[6] = 0x14; // len lo = 20
    pkt[7] = 0x00; // len hi = 0

    let payload = &mut pkt[8..28];
    if let Some(existing) = current_cfg {
        let copy_len = existing.len().min(20);
        payload[..copy_len].copy_from_slice(&existing[..copy_len]);
    }

    let sleep_secs: u16 = (sleep_minutes as u16) * 60;
    payload[2] = (sleep_secs & 0xFF) as u8;
    payload[3] = ((sleep_secs >> 8) & 0xFF) as u8;

    // Vendor magic trailer at bytes 18..20
    payload[18] = 0x5A;
    payload[19] = 0xA5;

    pkt
}

