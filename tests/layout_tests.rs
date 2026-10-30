use ajazz_rgb::layout::*;

#[test]
fn test_key_layout_integrity() {
    assert_eq!(KEYS.len(), PHYSICAL_KEY_COUNT);
    for key in KEYS {
        assert!(key.led_idx < TOTAL_LEDS, "Key {} LED idx {} exceeds {}", key.name, key.led_idx, TOTAL_LEDS);
        assert!(key.audio_band < 10, "Key {} audio band {} exceeds 10", key.name, key.audio_band);
        assert!(key.rect.w > 0.0 && key.rect.h > 0.0);
    }
}

#[test]
fn test_key_lookup() {
    let esc = find_key_by_name("esc").expect("esc key must exist");
    assert_eq!(esc.hid_code, 0x29);
    assert_eq!(esc.led_idx, 0);
    assert_eq!(esc.hw_idx, 0);

    let space = find_key_by_name("space").expect("space key must exist");
    assert_eq!(space.hid_code, 0x2c);
    assert_eq!(space.led_idx, 35);
    assert_eq!(space.hw_idx, 35);

    let by_led = find_key_by_led(0).expect("LED 0 must be Esc");
    assert_eq!(by_led.name, "esc");
}
