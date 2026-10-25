use ajazz_hid::protocol::*;
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::TOTAL_LEDS;

#[test]
fn test_shinetek_framing() {
    let pkt = build_shinetek_packet(0x07, &[0x06, 0x04, 0x03, 0x00, 0x07]);
    assert_eq!(pkt.len(), 64);
    assert_eq!(&pkt[0..4], &[0xAA, 0x55, 0xCC, 0x33]);
    assert_eq!(pkt[4], 0x07);
    assert_eq!(pkt[5], 0x06);
    assert_eq!(pkt[6], 0x04);
}

#[test]
fn test_shinetek_status_parser() {
    let mut resp = vec![0u8; 64];
    resp[0..4].copy_from_slice(&[0xAA, 0x55, 0xCC, 0x33]);
    resp[4] = CMD_STATUS;
    resp[5] = 85; // 85% battery
    resp[6] = 1;  // charging

    let status = parse_shinetek_status(&resp).expect("should parse status");
    assert_eq!(status.battery_percentage, Some(85));
    assert!(status.is_charging);
    assert_eq!(status.power_state, PowerState::Charging);
}

#[test]
fn test_bycombo_framing_and_checksum() {
    let payload = [1, 2, 3, 4, 5];
    let pkt = build_bycombo_packet(0x13, 0x88, 0, &payload);
    assert_eq!(pkt.len(), 20);
    assert_eq!(pkt[0], 0x13);
    assert_eq!(pkt[1], 0x15);
    assert_eq!(pkt[2], 0x88);
    assert_eq!(pkt[3], 0);

    // Verify modulo 256 sum checksum in byte 19
    let expected_sum: u8 = pkt[0..19].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    assert_eq!(pkt[19], expected_sum);
}

#[test]
fn test_bycombo_status_parser() {
    // 8-byte feature report from 3554:fa09 observed on hardware: 06 00 02 64 64 64 c9 6b
    let feature_buf = [0x06, 0x00, 0x02, 0x64, 0x64, 0x64, 0xc9, 0x6b];
    let status = parse_bycombo_status(&feature_buf).expect("should parse feature report");
    assert_eq!(status.battery_percentage, Some(100)); // 0x64 = 100
    assert_eq!(status.power_state, PowerState::OnBattery);
    assert!(!status.is_charging);
}

#[test]
fn test_rgb_matrix_chunks() {
    let mut matrix = [Color::BLACK; TOTAL_LEDS];
    matrix[0] = Color::RED;
    matrix[15] = Color::GREEN; // Esc key
    matrix[100] = Color::BLUE;

    let chunks = shinetek_rgb_chunks(&matrix, LIVE_CHUNK_BASE);
    assert_eq!(chunks.len(), 6);
    for chunk in &chunks {
        assert_eq!(chunk.len(), 64);
        assert_eq!(&chunk[0..4], &[0xAA, 0x55, 0xCC, 0x33]);
    }
}
