use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ColorParseError {
    #[error("invalid hex string length: {0}")]
    InvalidLength(usize),
    #[error("failed to parse hex color: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("unknown color name: {0}")]
    UnknownName(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255 };
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
    pub const YELLOW: Self = Self { r: 255, g: 255, b: 0 };
    pub const CYAN: Self = Self { r: 0, g: 255, b: 255 };
    pub const MAGENTA: Self = Self { r: 255, g: 0, b: 255 };
    pub const ORANGE: Self = Self { r: 255, g: 128, b: 0 };
    pub const PURPLE: Self = Self { r: 128, g: 0, b: 255 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert hue [0.0, 1.0], saturation [0.0, 1.0], value [0.0, 1.0] to RGB.
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h.fract();
        let h = if h < 0.0 { h + 1.0 } else { h };
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        if s <= 0.0 {
            let val = (v * 255.0).round() as u8;
            return Self::new(val, val, val);
        }

        let h_sector = h * 6.0;
        let sector = h_sector.floor() as i32;
        let f = h_sector - sector as f32;

        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));

        let (r, g, b) = match sector % 6 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };

        Self::new(
            (r * 255.0).round().clamp(0.0, 255.0) as u8,
            (g * 255.0).round().clamp(0.0, 255.0) as u8,
            (b * 255.0).round().clamp(0.0, 255.0) as u8,
        )
    }

    /// Linear interpolation between two colors.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self::new(
            (self.r as f32 * inv + other.r as f32 * t).round() as u8,
            (self.g as f32 * inv + other.g as f32 * t).round() as u8,
            (self.b as f32 * inv + other.b as f32 * t).round() as u8,
        )
    }

    /// Scale brightness by factor [0.0, 1.0].
    pub fn scale(self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self::new(
            (self.r as f32 * factor).round() as u8,
            (self.g as f32 * factor).round() as u8,
            (self.b as f32 * factor).round() as u8,
        )
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim().to_lowercase();
        match trimmed.as_str() {
            "black" => return Ok(Self::BLACK),
            "white" => return Ok(Self::WHITE),
            "red" => return Ok(Self::RED),
            "green" => return Ok(Self::GREEN),
            "blue" => return Ok(Self::BLUE),
            "yellow" => return Ok(Self::YELLOW),
            "cyan" => return Ok(Self::CYAN),
            "magenta" | "purple" => return Ok(Self::MAGENTA),
            "orange" => return Ok(Self::ORANGE),
            _ => {}
        }

        let hex = trimmed.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16)?;
            let g = u8::from_str_radix(&hex[2..4], 16)?;
            let b = u8::from_str_radix(&hex[4..6], 16)?;
            Ok(Self::new(r, g, b))
        } else if hex.len() == 3 {
            let r_str = format!("{}{}", &hex[0..1], &hex[0..1]);
            let g_str = format!("{}{}", &hex[1..2], &hex[1..2]);
            let b_str = format!("{}{}", &hex[2..3], &hex[2..3]);
            let r = u8::from_str_radix(&r_str, 16)?;
            let g = u8::from_str_radix(&g_str, 16)?;
            let b = u8::from_str_radix(&b_str, 16)?;
            Ok(Self::new(r, g, b))
        } else {
            Err(ColorParseError::InvalidLength(hex.len()))
        }
    }
}

/// Vendor hardware color palette index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PaletteColor {
    Red = 0,
    Green = 1,
    Blue = 2,
    Yellow = 3,
    Magenta = 4,
    Cyan = 5,
    White = 6,
    Rainbow = 7,
}

impl PaletteColor {
    pub fn to_color(self) -> Option<Color> {
        match self {
            Self::Red => Some(Color::RED),
            Self::Green => Some(Color::GREEN),
            Self::Blue => Some(Color::BLUE),
            Self::Yellow => Some(Color::YELLOW),
            Self::Magenta => Some(Color::MAGENTA),
            Self::Cyan => Some(Color::CYAN),
            Self::White => Some(Color::WHITE),
            Self::Rainbow => None,
        }
    }

    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Red,
            1 => Self::Green,
            2 => Self::Blue,
            3 => Self::Yellow,
            4 => Self::Magenta,
            5 => Self::Cyan,
            6 => Self::White,
            _ => Self::Rainbow,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Yellow => "yellow",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::Rainbow => "rainbow",
        }
    }

    pub fn closest(c: Color) -> Self {
        let presets = [
            (Self::Red, Color::RED),
            (Self::Green, Color::GREEN),
            (Self::Blue, Color::BLUE),
            (Self::Yellow, Color::YELLOW),
            (Self::Magenta, Color::MAGENTA),
            (Self::Cyan, Color::CYAN),
            (Self::White, Color::WHITE),
        ];
        let mut best = Self::Rainbow;
        let mut min_dist = u32::MAX;
        for (p, target) in presets {
            let dr = (c.r as i32 - target.r as i32).abs() as u32;
            let dg = (c.g as i32 - target.g as i32).abs() as u32;
            let db = (c.b as i32 - target.b as i32).abs() as u32;
            let dist = dr * dr + dg * dg + db * db;
            if dist < min_dist {
                min_dist = dist;
                best = p;
            }
        }
        best
    }
}
