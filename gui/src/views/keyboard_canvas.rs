use crate::theme::ThemeColors;
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::{KEYS, TOTAL_LEDS};
use egui::{
    pos2, vec2, Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui,
};
use std::collections::HashSet;
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PresetSelection {
    Wasd,
    Arrows,
    FunctionRow,
    NumberRow,
    Alphas,
    Modifiers,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientDirection {
    Horizontal,
    Vertical,
    Radial,
    Diagonal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PresetTheme {
    Cyberpunk,
    Synthwave,
    ThermalFlame,
    OceanicIce,
    MatrixTerminal,
    GamerWasd,
}

pub struct KeyboardCanvasState {
    /// Currently selected key names
    pub selected_keys: HashSet<&'static str>,
    /// Per-key custom painted colors indexed by key.hw_idx (0..108)
    pub custom_colors: [Color; TOTAL_LEDS],
    /// Marquee drag selection box start position
    drag_start: Option<Pos2>,
    /// Gradient Generator State
    pub gradient_c1: Color,
    pub gradient_c2: Color,
    pub gradient_c3: Option<Color>,
    pub gradient_dir: GradientDirection,
    pub gradient_selected_only: bool,
    /// Live Mirroring toggle: immediately push to physical hardware
    pub live_mirror: bool,
    /// Internal PRNG state
    rng_state: u32,
}

impl Default for KeyboardCanvasState {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros() as u32)
            .unwrap_or(0xCAFE_BABE);

        Self {
            selected_keys: HashSet::new(),
            custom_colors: [Color::new(0, 242, 254); TOTAL_LEDS],
            drag_start: None,
            gradient_c1: Color::new(0, 242, 254),    // Cyan
            gradient_c2: Color::new(255, 0, 127),   // Neon Pink
            gradient_c3: None,
            gradient_dir: GradientDirection::Horizontal,
            gradient_selected_only: false,
            live_mirror: true,
            rng_state: seed,
        }
    }
}

impl KeyboardCanvasState {
    pub fn next_rand(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng_state >> 8) as f32 / 16777216.0
    }

    pub fn select_preset(&mut self, preset: PresetSelection) {
        self.selected_keys.clear();
        for key in KEYS {
            let matches = match preset {
                PresetSelection::Wasd => matches!(key.name, "w" | "a" | "s" | "d"),
                PresetSelection::Arrows => matches!(key.name, "up" | "down" | "left" | "right"),
                PresetSelection::FunctionRow => key.row == 0,
                PresetSelection::NumberRow => key.row == 1,
                PresetSelection::Alphas => {
                    key.row >= 1
                        && key.row <= 4
                        && key.col >= 1
                        && !matches!(
                            key.name,
                            "backspace"
                                | "tab"
                                | "capslock"
                                | "enter"
                                | "lshift"
                                | "rshift"
                                | "home"
                                | "pgup"
                                | "pgdn"
                                | "end"
                                | "up"
                        )
                }
                PresetSelection::Modifiers => matches!(
                    key.name,
                    "lctrl" | "rctrl" | "lalt" | "ralt" | "lwin" | "fn" | "lshift" | "rshift"
                ),
                PresetSelection::All => true,
            };
            if matches {
                self.selected_keys.insert(key.name);
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_keys.clear();
    }

    pub fn paint_selected(&mut self, color: Color) {
        let has_selection = !self.selected_keys.is_empty();
        for key in KEYS {
            if (!has_selection || self.selected_keys.contains(key.name)) && key.hw_idx < TOTAL_LEDS {
                self.custom_colors[key.hw_idx] = color;
            }
        }
    }

    pub fn randomize_all(&mut self) {
        for key in KEYS {
            let h = self.next_rand();
            let s = 0.88 + self.next_rand() * 0.12;
            let v = 0.98;
            self.custom_colors[key.hw_idx] = Color::from_hsv(h, s, v);
        }
    }

    pub fn randomize_selected(&mut self) {
        let has_selection = !self.selected_keys.is_empty();
        for key in KEYS {
            if !has_selection || self.selected_keys.contains(key.name) {
                let h = self.next_rand();
                let s = 0.88 + self.next_rand() * 0.12;
                let v = 0.98;
                self.custom_colors[key.hw_idx] = Color::from_hsv(h, s, v);
            }
        }
    }

    pub fn apply_spatial_gradient(
        &mut self,
        c1: Color,
        c2: Color,
        c3: Option<Color>,
        dir: GradientDirection,
        selected_only: bool,
    ) {
        let has_selection = selected_only && !self.selected_keys.is_empty();

        let min_x = 29.0f32;
        let max_x = 554.0f32;
        let min_y = 41.0f32;
        let max_y = 237.0f32;
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let max_d = ((max_x - cx).powi(2) + (max_y - cy).powi(2)).sqrt();

        for key in KEYS {
            if has_selection && !self.selected_keys.contains(key.name) {
                continue;
            }

            let kx = key.rect.center_x();
            let ky = key.rect.center_y();

            let t = match dir {
                GradientDirection::Horizontal => ((kx - min_x) / (max_x - min_x)).clamp(0.0, 1.0),
                GradientDirection::Vertical => ((ky - min_y) / (max_y - min_y)).clamp(0.0, 1.0),
                GradientDirection::Radial => {
                    let d = ((kx - cx).powi(2) + (ky - cy).powi(2)).sqrt();
                    (d / max_d).clamp(0.0, 1.0)
                }
                GradientDirection::Diagonal => {
                    let tx = (kx - min_x) / (max_x - min_x);
                    let ty = (ky - min_y) / (max_y - min_y);
                    ((tx + ty) * 0.5).clamp(0.0, 1.0)
                }
            };

            let final_color = match c3 {
                Some(stop3) => {
                    if t < 0.5 {
                        c1.lerp(c2, t * 2.0)
                    } else {
                        c2.lerp(stop3, (t - 0.5) * 2.0)
                    }
                }
                None => c1.lerp(c2, t),
            };

            self.custom_colors[key.hw_idx] = final_color;
        }
    }

    pub fn randomize_gradient(&mut self, selected_only: bool) {
        let h1 = self.next_rand();
        let h2 = (h1 + 0.3 + self.next_rand() * 0.4) % 1.0;
        let c1 = Color::from_hsv(h1, 0.95, 1.0);
        let c2 = Color::from_hsv(h2, 0.95, 1.0);
        let use_three = self.next_rand() > 0.4;
        let c3 = if use_three {
            let h3 = (h2 + 0.3 + self.next_rand() * 0.4) % 1.0;
            Some(Color::from_hsv(h3, 0.95, 1.0))
        } else {
            None
        };

        let dir = match (self.next_rand() * 4.0) as u32 % 4 {
            0 => GradientDirection::Horizontal,
            1 => GradientDirection::Vertical,
            2 => GradientDirection::Radial,
            _ => GradientDirection::Diagonal,
        };

        self.gradient_c1 = c1;
        self.gradient_c2 = c2;
        self.gradient_c3 = c3;
        self.gradient_dir = dir;

        self.apply_spatial_gradient(c1, c2, c3, dir, selected_only);
    }

    pub fn apply_preset_theme(&mut self, theme: PresetTheme) {
        match theme {
            PresetTheme::Cyberpunk => {
                let pink = Color::new(255, 0, 127);
                let cyan = Color::new(0, 242, 254);
                self.gradient_c1 = pink;
                self.gradient_c2 = cyan;
                self.gradient_c3 = None;
                self.gradient_dir = GradientDirection::Horizontal;
                self.apply_spatial_gradient(pink, cyan, None, GradientDirection::Horizontal, false);
            }
            PresetTheme::Synthwave => {
                let violet = Color::new(139, 0, 255);
                let magenta = Color::new(255, 20, 147);
                let orange = Color::new(255, 140, 0);
                self.gradient_c1 = violet;
                self.gradient_c2 = magenta;
                self.gradient_c3 = Some(orange);
                self.gradient_dir = GradientDirection::Diagonal;
                self.apply_spatial_gradient(violet, magenta, Some(orange), GradientDirection::Diagonal, false);
            }
            PresetTheme::ThermalFlame => {
                let dark_red = Color::new(180, 0, 0);
                let orange = Color::new(255, 69, 0);
                let gold = Color::new(255, 215, 0);
                self.gradient_c1 = dark_red;
                self.gradient_c2 = orange;
                self.gradient_c3 = Some(gold);
                self.gradient_dir = GradientDirection::Vertical;
                self.apply_spatial_gradient(dark_red, orange, Some(gold), GradientDirection::Vertical, false);
            }
            PresetTheme::OceanicIce => {
                let navy = Color::new(0, 31, 63);
                let aqua = Color::new(0, 201, 255);
                let mint = Color::new(146, 254, 157);
                self.gradient_c1 = navy;
                self.gradient_c2 = aqua;
                self.gradient_c3 = Some(mint);
                self.gradient_dir = GradientDirection::Horizontal;
                self.apply_spatial_gradient(navy, aqua, Some(mint), GradientDirection::Horizontal, false);
            }
            PresetTheme::MatrixTerminal => {
                let dark_green = Color::new(0, 45, 0);
                let bright_lime = Color::new(0, 255, 65);
                self.gradient_c1 = dark_green;
                self.gradient_c2 = bright_lime;
                self.gradient_c3 = None;
                self.gradient_dir = GradientDirection::Vertical;
                self.apply_spatial_gradient(dark_green, bright_lime, None, GradientDirection::Vertical, false);
            }
            PresetTheme::GamerWasd => {
                let wasd_red = Color::new(255, 0, 50);
                let arrow_gold = Color::new(255, 215, 0);
                let alpha_cyan = Color::new(0, 212, 255);
                let mod_purple = Color::new(138, 43, 226);

                for key in KEYS {
                    let col = if matches!(key.name, "w" | "a" | "s" | "d") {
                        wasd_red
                    } else if matches!(key.name, "up" | "down" | "left" | "right") {
                        arrow_gold
                    } else if matches!(key.name, "lctrl" | "rctrl" | "lalt" | "ralt" | "lwin" | "fn" | "lshift" | "rshift" | "esc" | "enter" | "backspace" | "tab" | "capslock") {
                        mod_purple
                    } else {
                        alpha_cyan
                    };
                    self.custom_colors[key.hw_idx] = col;
                }
            }
        }
    }
}

/// Renders the interactive 75% 82-key AK820 Max visual keyboard canvas.
/// Returns `true` if any keys were clicked, painted, or selection modified.
pub fn render_keyboard_canvas(
    ui: &mut Ui,
    state: &mut KeyboardCanvasState,
    preview_frame: &[Color; TOTAL_LEDS],
    _audio_levels: Option<&[f32]>,
    is_custom_mode: bool,
) -> bool {
    let mut changed = false;

    ui.vertical(|ui| {
        // Preset toolbar
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("Quick Select:")
                    .color(ThemeColors::TEXT_SECONDARY)
                    .strong(),
            );

            if ui
                .add(egui::Button::new("WASD").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::Wasd);
            }
            if ui
                .add(egui::Button::new("Arrows").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::Arrows);
            }
            if ui
                .add(egui::Button::new("Fn Row").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::FunctionRow);
            }
            if ui
                .add(egui::Button::new("Numbers").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::NumberRow);
            }
            if ui
                .add(egui::Button::new("Alphas").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::Alphas);
            }
            if ui
                .add(egui::Button::new("Modifiers").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::Modifiers);
            }
            if ui
                .add(egui::Button::new("All Keys").rounding(Rounding::same(6.0)))
                .clicked()
            {
                state.select_preset(PresetSelection::All);
            }
            if ui
                .add(
                    egui::Button::new("Clear")
                        .fill(ThemeColors::BG_MAIN)
                        .rounding(Rounding::same(6.0)),
                )
                .clicked()
            {
                state.clear_selection();
            }

            ui.separator();
            ui.label(
                egui::RichText::new(format!("Selected: {} keys", state.selected_keys.len()))
                    .color(ThemeColors::ACCENT_CYAN),
            );
        });

        ui.add_space(8.0);

        // Layout Dimensions
        // Canonical canvas coordinates: width 590.0, height 265.0
        let available_width = ui.available_width().max(320.0);
        let canonical_w = 590.0f32;
        let canonical_h = 265.0f32;

        let scale = (available_width / canonical_w).min(1.8).max(0.5);
        let canvas_size = vec2(canonical_w * scale, canonical_h * scale);

        let (canvas_rect, response) =
            ui.allocate_exact_size(canvas_size, Sense::click_and_drag());
        let painter = ui.painter_at(canvas_rect);

        // Pointer input handling
        let pointer_pos = response.interact_pointer_pos();

        if response.drag_started() {
            state.drag_start = pointer_pos;
        }

        let mut marquee_rect: Option<Rect> = None;
        if response.dragged() {
            if let (Some(start), Some(curr)) = (state.drag_start, pointer_pos) {
                marquee_rect = Some(Rect::from_two_pos(start, curr));
            }
        }

        if response.drag_stopped() {
            if let Some(m_rect) = marquee_rect {
                for key in KEYS {
                    let kx = canvas_rect.min.x + (key.rect.x - 20.0) * scale;
                    let ky = canvas_rect.min.y + (key.rect.y - 30.0) * scale;
                    let kw = key.rect.w * scale;
                    let kh = key.rect.h * scale;
                    let k_rect = Rect::from_min_size(pos2(kx, ky), vec2(kw, kh));

                    if m_rect.intersects(k_rect) {
                        state.selected_keys.insert(key.name);
                    }
                }
            }
            state.drag_start = None;
        }

        // Draw Keyboard Case (Chassis)
        let chassis_rounding = Rounding::same(16.0 * scale);
        // Subtle outer glow
        painter.rect_filled(
            canvas_rect.expand(2.0 * scale),
            chassis_rounding,
            Color32::from_rgba_premultiplied(0, 242, 254, 18),
        );
        // Bezel
        painter.rect(
            canvas_rect,
            chassis_rounding,
            ThemeColors::BG_PANEL,
            Stroke::new(2.0 * scale, ThemeColors::BORDER_MUTED),
        );
        // Inner tray
        let inner_rect = canvas_rect.shrink(6.0 * scale);
        painter.rect(
            inner_rect,
            Rounding::same(12.0 * scale),
            Color32::from_rgb(11, 16, 26),
            Stroke::new(1.0 * scale, ThemeColors::BORDER_SUBTLE),
        );

        // Rotary Volume Knob at Top-Right
        let knob_center = pos2(
            canvas_rect.min.x + (546.0) * scale,
            canvas_rect.min.y + (22.0) * scale,
        );
        let knob_radius = 11.0 * scale;
        painter.circle(
            knob_center,
            knob_radius,
            ThemeColors::BG_CARD,
            Stroke::new(1.5 * scale, ThemeColors::ACCENT_CYAN),
        );
        painter.circle_filled(
            pos2(knob_center.x, knob_center.y - 5.0 * scale),
            2.0 * scale,
            ThemeColors::ACCENT_CYAN,
        );

        // Click detection on individual keys
        let clicked = response.clicked();

        // Draw Keys
        for key in KEYS {
            let kx = canvas_rect.min.x + (key.rect.x - 20.0) * scale;
            let ky = canvas_rect.min.y + (key.rect.y - 30.0) * scale;
            let kw = key.rect.w * scale;
            let kh = key.rect.h * scale;
            let key_rect = Rect::from_min_size(pos2(kx, ky), vec2(kw, kh));

            let is_selected = state.selected_keys.contains(key.name);
            let is_hovered = pointer_pos.map_or(false, |p| key_rect.contains(p));

            if clicked && is_hovered {
                if is_selected {
                    state.selected_keys.remove(key.name);
                } else {
                    state.selected_keys.insert(key.name);
                }
                changed = true;
            }

            // Determine key illumination color using exact hw_idx matrix
            let led_color = if is_custom_mode {
                state.custom_colors[key.hw_idx]
            } else {
                preview_frame[key.hw_idx]
            };

            let egui_led = Color32::from_rgb(led_color.r, led_color.g, led_color.b);

            // Keycap body color
            let keycap_fill = if is_selected {
                ThemeColors::BG_KEYCAP_SELECTED
            } else if is_hovered {
                ThemeColors::BG_CARD_HOVER
            } else {
                ThemeColors::BG_KEYCAP_IDLE
            };

            let rounding = Rounding::same(4.0 * scale);

            // Keycap base
            painter.rect_filled(key_rect, rounding, keycap_fill);

            // Underglow / backlight bar
            let underglow_rect = Rect::from_min_size(
                pos2(kx + 2.0 * scale, ky + 2.0 * scale),
                vec2(kw - 4.0 * scale, 3.0 * scale),
            );
            painter.rect_filled(underglow_rect, Rounding::same(1.5 * scale), egui_led);

            // Keycap border & selection halo
            let stroke = if is_selected {
                Stroke::new(1.8 * scale, ThemeColors::ACCENT_CYAN)
            } else if is_hovered {
                Stroke::new(1.2 * scale, Color32::WHITE)
            } else {
                Stroke::new(1.0 * scale, ThemeColors::BORDER_MUTED)
            };
            painter.rect_stroke(key_rect, rounding, stroke);

            // Key Legend Text
            let text_color = if is_selected {
                ThemeColors::ACCENT_CYAN
            } else {
                ThemeColors::TEXT_PRIMARY
            };

            let font_size = if key.label.len() > 3 {
                (8.5 * scale).max(7.0)
            } else {
                (10.0 * scale).max(8.0)
            };

            painter.text(
                key_rect.center(),
                egui::Align2::CENTER_CENTER,
                key.label,
                egui::FontId::proportional(font_size),
                text_color,
            );
        }

        // Draw marquee drag box if active
        if let Some(m_rect) = marquee_rect {
            painter.rect(
                m_rect,
                Rounding::same(2.0),
                Color32::from_rgba_premultiplied(0, 242, 254, 30),
                Stroke::new(1.5_f32, ThemeColors::ACCENT_CYAN),
            );
        }
    });

    changed
}
