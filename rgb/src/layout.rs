use serde::{Deserialize, Serialize};

pub const TOTAL_LEDS: usize = 108;
pub const PHYSICAL_KEY_COUNT: usize = 82;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.w * 0.5
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.h * 0.5
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KeyDef {
    pub hid_code: u8,
    pub name: &'static str,
    pub label: &'static str,
    pub led_idx: usize,
    pub row: usize,
    pub col: usize,
    pub rect: Rect,
    /// Audio frequency band index (0..9) matching AK820 audiobar.txt
    pub audio_band: usize,
}

/// The 82 physical keys of the Ajazz AK820 Max keyboard.
/// Derived from vendor layout kb-k600t.xml and audiobar.txt.
pub static KEYS: &[KeyDef] = &[
    // Row 0 (Function row)
    KeyDef { hid_code: 0x29, name: "esc", label: "Esc", led_idx: 15, row: 0, col: 0, rect: Rect::new(29.0, 41.0, 34.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x3a, name: "f1", label: "F1", led_idx: 14, row: 0, col: 1, rect: Rect::new(71.0, 41.0, 34.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x3b, name: "f2", label: "F2", led_idx: 13, row: 0, col: 2, rect: Rect::new(103.0, 41.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x3c, name: "f3", label: "F3", led_idx: 12, row: 0, col: 3, rect: Rect::new(136.0, 41.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x3d, name: "f4", label: "F4", led_idx: 11, row: 0, col: 4, rect: Rect::new(169.0, 41.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x3e, name: "f5", label: "F5", led_idx: 10, row: 0, col: 5, rect: Rect::new(210.0, 41.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x3f, name: "f6", label: "F6", led_idx: 9, row: 0, col: 6, rect: Rect::new(243.0, 41.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x40, name: "f7", label: "F7", led_idx: 8, row: 0, col: 7, rect: Rect::new(276.0, 41.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x41, name: "f8", label: "F8", led_idx: 7, row: 0, col: 8, rect: Rect::new(307.0, 41.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x42, name: "f9", label: "F9", led_idx: 6, row: 0, col: 9, rect: Rect::new(350.0, 41.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x43, name: "f10", label: "F10", led_idx: 5, row: 0, col: 10, rect: Rect::new(383.0, 41.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x44, name: "f11", label: "F11", led_idx: 4, row: 0, col: 11, rect: Rect::new(416.0, 41.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x45, name: "f12", label: "F12", led_idx: 3, row: 0, col: 12, rect: Rect::new(448.0, 41.0, 34.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x4c, name: "del", label: "Del", led_idx: 43, row: 0, col: 13, rect: Rect::new(489.0, 41.0, 34.0, 34.0), audio_band: 8 },

    // Row 1 (Number row)
    KeyDef { hid_code: 0x35, name: "grave", label: "`~", led_idx: 16, row: 1, col: 0, rect: Rect::new(30.0, 81.0, 34.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x1e, name: "1", label: "1!", led_idx: 17, row: 1, col: 1, rect: Rect::new(63.0, 81.0, 34.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x1f, name: "2", label: "2@", led_idx: 18, row: 1, col: 2, rect: Rect::new(95.0, 81.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x20, name: "3", label: "3#", led_idx: 19, row: 1, col: 3, rect: Rect::new(128.0, 81.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x21, name: "4", label: "4$", led_idx: 20, row: 1, col: 4, rect: Rect::new(161.0, 81.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x22, name: "5", label: "5%", led_idx: 21, row: 1, col: 5, rect: Rect::new(194.0, 81.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x23, name: "6", label: "6^", led_idx: 22, row: 1, col: 6, rect: Rect::new(227.0, 81.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x24, name: "7", label: "7&", led_idx: 23, row: 1, col: 7, rect: Rect::new(260.0, 81.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x25, name: "8", label: "8*", led_idx: 24, row: 1, col: 8, rect: Rect::new(292.0, 81.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x26, name: "9", label: "9(", led_idx: 25, row: 1, col: 9, rect: Rect::new(325.0, 81.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x27, name: "0", label: "0)", led_idx: 26, row: 1, col: 10, rect: Rect::new(358.0, 81.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x2d, name: "minus", label: "-_", led_idx: 27, row: 1, col: 11, rect: Rect::new(390.0, 81.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x2e, name: "equal", label: "=+", led_idx: 28, row: 1, col: 12, rect: Rect::new(424.0, 81.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x2a, name: "backspace", label: "Backspace", led_idx: 29, row: 1, col: 13, rect: Rect::new(457.0, 81.0, 65.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x4a, name: "home", label: "Home", led_idx: 31, row: 1, col: 14, rect: Rect::new(537.0, 81.0, 34.0, 34.0), audio_band: 9 },

    // Row 2 (QWERTY row)
    KeyDef { hid_code: 0x2b, name: "tab", label: "Tab", led_idx: 57, row: 2, col: 0, rect: Rect::new(30.0, 114.0, 51.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x14, name: "q", label: "Q", led_idx: 56, row: 2, col: 1, rect: Rect::new(79.0, 114.0, 34.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x1a, name: "w", label: "W", led_idx: 55, row: 2, col: 2, rect: Rect::new(112.0, 114.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x08, name: "e", label: "E", led_idx: 54, row: 2, col: 3, rect: Rect::new(144.0, 114.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x15, name: "r", label: "R", led_idx: 53, row: 2, col: 4, rect: Rect::new(177.0, 114.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x17, name: "t", label: "T", led_idx: 52, row: 2, col: 5, rect: Rect::new(210.0, 114.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x1c, name: "y", label: "Y", led_idx: 51, row: 2, col: 6, rect: Rect::new(243.0, 114.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x18, name: "u", label: "U", led_idx: 50, row: 2, col: 7, rect: Rect::new(276.0, 114.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x0c, name: "i", label: "I", led_idx: 49, row: 2, col: 8, rect: Rect::new(309.0, 114.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x12, name: "o", label: "O", led_idx: 48, row: 2, col: 9, rect: Rect::new(341.0, 114.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x13, name: "p", label: "P", led_idx: 47, row: 2, col: 10, rect: Rect::new(375.0, 114.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x2f, name: "lbracket", label: "[{", led_idx: 46, row: 2, col: 11, rect: Rect::new(407.0, 114.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x30, name: "rbracket", label: "]}", led_idx: 45, row: 2, col: 12, rect: Rect::new(439.0, 114.0, 34.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x31, name: "backslash", label: "\\|", led_idx: 44, row: 2, col: 13, rect: Rect::new(473.0, 114.0, 51.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x4b, name: "pgup", label: "PgUp", led_idx: 32, row: 2, col: 14, rect: Rect::new(537.0, 114.0, 34.0, 34.0), audio_band: 9 },

    // Row 3 (ASDF row)
    KeyDef { hid_code: 0x39, name: "capslock", label: "CapsLock", led_idx: 58, row: 3, col: 0, rect: Rect::new(30.0, 148.0, 54.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x04, name: "a", label: "A", led_idx: 59, row: 3, col: 1, rect: Rect::new(88.0, 148.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x16, name: "s", label: "S", led_idx: 60, row: 3, col: 2, rect: Rect::new(120.0, 148.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x07, name: "d", label: "D", led_idx: 61, row: 3, col: 3, rect: Rect::new(153.0, 148.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x09, name: "f", label: "F", led_idx: 62, row: 3, col: 4, rect: Rect::new(186.0, 148.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x0a, name: "g", label: "G", led_idx: 63, row: 3, col: 5, rect: Rect::new(219.0, 148.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x0b, name: "h", label: "H", led_idx: 64, row: 3, col: 6, rect: Rect::new(252.0, 148.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x0d, name: "j", label: "J", led_idx: 65, row: 3, col: 7, rect: Rect::new(284.0, 148.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x0e, name: "k", label: "K", led_idx: 66, row: 3, col: 8, rect: Rect::new(317.0, 148.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x0f, name: "l", label: "L", led_idx: 67, row: 3, col: 9, rect: Rect::new(351.0, 148.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x33, name: "semicolon", label: ";:", led_idx: 68, row: 3, col: 10, rect: Rect::new(383.0, 148.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x34, name: "quote", label: "'\"", led_idx: 69, row: 3, col: 11, rect: Rect::new(416.0, 148.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0x28, name: "enter", label: "Enter", led_idx: 70, row: 3, col: 12, rect: Rect::new(449.0, 148.0, 74.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x4e, name: "pgdn", label: "PgDn", led_idx: 41, row: 3, col: 13, rect: Rect::new(537.0, 146.0, 34.0, 34.0), audio_band: 9 },

    // Row 4 (ZXCV row)
    KeyDef { hid_code: 0xe1, name: "lshift", label: "Shift", led_idx: 90, row: 4, col: 0, rect: Rect::new(30.0, 180.0, 74.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0x1d, name: "z", label: "Z", led_idx: 89, row: 4, col: 1, rect: Rect::new(103.0, 180.0, 34.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x1b, name: "x", label: "X", led_idx: 88, row: 4, col: 2, rect: Rect::new(137.0, 180.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x06, name: "c", label: "C", led_idx: 87, row: 4, col: 3, rect: Rect::new(169.0, 180.0, 34.0, 34.0), audio_band: 2 },
    KeyDef { hid_code: 0x19, name: "v", label: "V", led_idx: 86, row: 4, col: 4, rect: Rect::new(202.0, 180.0, 34.0, 34.0), audio_band: 3 },
    KeyDef { hid_code: 0x05, name: "b", label: "B", led_idx: 85, row: 4, col: 5, rect: Rect::new(235.0, 180.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x11, name: "n", label: "N", led_idx: 84, row: 4, col: 6, rect: Rect::new(267.0, 180.0, 34.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0x10, name: "m", label: "M", led_idx: 83, row: 4, col: 7, rect: Rect::new(300.0, 180.0, 34.0, 34.0), audio_band: 5 },
    KeyDef { hid_code: 0x36, name: "comma", label: ",<", led_idx: 82, row: 4, col: 8, rect: Rect::new(333.0, 180.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x37, name: "period", label: ".>", led_idx: 81, row: 4, col: 9, rect: Rect::new(366.0, 180.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0x38, name: "slash", label: "/?", led_idx: 80, row: 4, col: 10, rect: Rect::new(399.0, 180.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0xe5, name: "rshift", label: "Shift", led_idx: 79, row: 4, col: 11, rect: Rect::new(432.0, 180.0, 54.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x52, name: "up", label: "↑", led_idx: 78, row: 4, col: 12, rect: Rect::new(497.0, 186.0, 34.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x4d, name: "end", label: "End", led_idx: 42, row: 4, col: 13, rect: Rect::new(537.0, 180.0, 34.0, 34.0), audio_band: 9 },

    // Row 5 (Bottom row)
    KeyDef { hid_code: 0xe0, name: "lctrl", label: "Ctrl", led_idx: 91, row: 5, col: 0, rect: Rect::new(31.0, 212.0, 42.0, 34.0), audio_band: 0 },
    KeyDef { hid_code: 0xe3, name: "lwin", label: "Win", led_idx: 92, row: 5, col: 1, rect: Rect::new(71.0, 212.0, 42.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0xe2, name: "lalt", label: "Alt", led_idx: 93, row: 5, col: 2, rect: Rect::new(111.0, 212.0, 42.0, 34.0), audio_band: 1 },
    KeyDef { hid_code: 0x2c, name: "space", label: "Space", led_idx: 94, row: 5, col: 3, rect: Rect::new(153.0, 212.0, 208.0, 34.0), audio_band: 4 },
    KeyDef { hid_code: 0xe6, name: "ralt", label: "Alt", led_idx: 95, row: 5, col: 4, rect: Rect::new(357.0, 212.0, 34.0, 34.0), audio_band: 6 },
    KeyDef { hid_code: 0xaf, name: "fn", label: "Fn", led_idx: 97, row: 5, col: 5, rect: Rect::new(390.0, 212.0, 34.0, 34.0), audio_band: 7 },
    KeyDef { hid_code: 0xe4, name: "rctrl", label: "Ctrl", led_idx: 98, row: 5, col: 6, rect: Rect::new(424.0, 212.0, 34.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x50, name: "left", label: "←", led_idx: 99, row: 5, col: 7, rect: Rect::new(464.0, 220.0, 31.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x51, name: "down", label: "↓", led_idx: 100, row: 5, col: 8, rect: Rect::new(496.0, 220.0, 31.0, 34.0), audio_band: 8 },
    KeyDef { hid_code: 0x4f, name: "right", label: "→", led_idx: 101, row: 5, col: 9, rect: Rect::new(530.0, 220.0, 31.0, 34.0), audio_band: 9 },
];

pub fn find_key_by_name(name: &str) -> Option<&'static KeyDef> {
    let lower = name.to_lowercase();
    KEYS.iter().find(|k| k.name == lower || k.label.to_lowercase() == lower)
}

pub fn find_key_by_led(led_idx: usize) -> Option<&'static KeyDef> {
    KEYS.iter().find(|k| k.led_idx == led_idx)
}

pub fn find_key_by_hid(hid_code: u8) -> Option<&'static KeyDef> {
    KEYS.iter().find(|k| k.hid_code == hid_code)
}
