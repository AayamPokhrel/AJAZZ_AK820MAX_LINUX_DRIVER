use ajazz_hid::protocol::*;
use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{Direction, EffectMode, EffectSettings};
use ajazz_rgb::layout::TOTAL_LEDS;
use ajazz_rgb::storage::PersistentConfig;

#[test]
fn test_persistent_config_roundtrip() {
    let mut cfg = PersistentConfig::default();
    cfg.active_category = "canvas".to_string();
    cfg.lighting.mode = EffectMode::SineWave;
    cfg.lighting.brightness = 4;
    cfg.lighting.speed = 3;
    cfg.lighting.direction = Direction::Left;
    cfg.lighting.custom_color = Some(Color::new(255, 69, 0));
    cfg.canvas.is_active = true;
    cfg.canvas.brightness = 5;
    cfg.canvas.colors[0] = "#FF007F".to_string();
    cfg.device.sleep_minutes = 10;

    let json = serde_json::to_string_pretty(&cfg).expect("Must serialize to JSON");
    let loaded: PersistentConfig = serde_json::from_str(&json).expect("Must deserialize from JSON");

    assert_eq!(cfg, loaded);
    assert_eq!(loaded.active_category, "canvas");
    assert_eq!(loaded.lighting.mode, EffectMode::SineWave);
    assert_eq!(loaded.lighting.custom_color, Some(Color::new(255, 69, 0)));
    assert_eq!(loaded.canvas.colors[0], "#FF007F");
    assert_eq!(loaded.device.sleep_minutes, 10);
}

#[test]
fn test_palette_closest_matcher() {
    assert_eq!(PaletteColor::closest(Color::new(255, 0, 0)), PaletteColor::Red);
    assert_eq!(PaletteColor::closest(Color::new(0, 255, 0)), PaletteColor::Green);
    assert_eq!(PaletteColor::closest(Color::new(0, 0, 255)), PaletteColor::Blue);
    assert_eq!(PaletteColor::closest(Color::new(0, 255, 255)), PaletteColor::Cyan);
    assert_eq!(PaletteColor::closest(Color::new(255, 255, 0)), PaletteColor::Yellow);
    assert_eq!(PaletteColor::closest(Color::new(255, 0, 255)), PaletteColor::Magenta);
    assert_eq!(PaletteColor::closest(Color::new(255, 255, 255)), PaletteColor::White);
    assert_eq!(PaletteColor::closest(Color::new(255, 50, 50)), PaletteColor::Red);
    assert_eq!(PaletteColor::closest(Color::new(0, 240, 250)), PaletteColor::Cyan);
}

#[test]
fn test_effect_mode_hardware_id_mappings() {
    assert_eq!(EffectMode::Static.mode_id(), 1);
    assert_eq!(EffectMode::Spectrum.mode_id(), 2);
    assert_eq!(EffectMode::Breathing.mode_id(), 3);
    assert_eq!(EffectMode::Custom.mode_id(), 4);
    assert_eq!(EffectMode::Windmill.mode_id(), 5);
    assert_eq!(EffectMode::SineWave.mode_id(), 6);
    assert_eq!(EffectMode::Screen.mode_id(), 7);
    assert_eq!(EffectMode::TopBottom.mode_id(), 8);
    assert_eq!(EffectMode::Galloping.mode_id(), 9);
    assert_eq!(EffectMode::Rain.mode_id(), 10);
    assert_eq!(EffectMode::RippleLr.mode_id(), 11);
    assert_eq!(EffectMode::Cross.mode_id(), 12);
    assert_eq!(EffectMode::KeyBreathing.mode_id(), 13);
    assert_eq!(EffectMode::Starry.mode_id(), 14);
    assert_eq!(EffectMode::Ejection.mode_id(), 15);
    assert_eq!(EffectMode::Kaleidoscope.mode_id(), 16);
    assert_eq!(EffectMode::Music.mode_id(), 17);
    assert_eq!(EffectMode::Scroll.mode_id(), 18);
    assert_eq!(EffectMode::Off.mode_id(), 0);

    // Aliases
    assert_eq!(EffectMode::from_name("Rainbow"), Some(EffectMode::Spectrum));
    assert_eq!(EffectMode::from_name("Respire"), Some(EffectMode::Breathing));
    assert_eq!(EffectMode::from_name("Waterfall"), Some(EffectMode::TopBottom));
    assert_eq!(EffectMode::from_name("Storm"), Some(EffectMode::Galloping));
    assert_eq!(EffectMode::from_name("Raindrops"), Some(EffectMode::Rain));
    assert_eq!(EffectMode::from_name("Stars_twinkle"), Some(EffectMode::Starry));
    assert_eq!(EffectMode::from_name("Lights Off"), Some(EffectMode::Off));
}

#[test]
fn test_live_music_packet_framing() {
    let mut matrix = [Color::BLACK; TOTAL_LEDS];
    matrix[0] = Color::new(255, 0, 0);   // Key 0 Red
    matrix[1] = Color::new(0, 255, 0);   // Key 1 Green
    matrix[2] = Color::new(0, 0, 255);   // Key 2 Blue

    let pkt = bycombo_wired_live_music_packet(&matrix);
    assert_eq!(pkt.len(), 520);
    assert_eq!(pkt[0], 0x06); // Report ID 6
    assert_eq!(pkt[1], 0x08); // cmd = 0x08
    assert_eq!(pkt[2], 0x01); // Write
    assert_eq!(pkt[4], 0x01); // 1 chunk
    assert_eq!(pkt[6], 0x7A); // 378 bytes total length (0x017A)
    assert_eq!(pkt[7], 0x01);

    // Key 0 RGB at offset 8
    assert_eq!(pkt[8], 255);
    assert_eq!(pkt[9], 0);
    assert_eq!(pkt[10], 0);

    // Key 1 RGB at offset 11
    assert_eq!(pkt[11], 0);
    assert_eq!(pkt[12], 255);
    assert_eq!(pkt[13], 0);

    // Key 2 RGB at offset 14
    assert_eq!(pkt[14], 0);
    assert_eq!(pkt[15], 0);
    assert_eq!(pkt[16], 255);
}

#[test]
fn test_bycombo_wired_set_led_packet_format() {
    let settings = EffectSettings {
        mode: EffectMode::SineWave,
        brightness: 5,
        speed: 3,
        direction: Direction::Left,
        palette_color: PaletteColor::Rainbow,
        custom_color: None,
    };

    let pkt = bycombo_wired_set_led_packet(None, &settings);
    assert_eq!(pkt.len(), 520);
    assert_eq!(pkt[0], 0x06); // Report ID 6
    assert_eq!(pkt[1], 0x04); // cmd = 0x04 (SetLED)
    assert_eq!(pkt[2], 0x01); // Write
    assert_eq!(pkt[6], 0x80); // 128 bytes length
    assert_eq!(pkt[8 + 10], 6); // hw = 6 (SineWave)
    assert_eq!(pkt[8 + 126], 0x5A); // magic trailer
    assert_eq!(pkt[8 + 127], 0xA5);
}
