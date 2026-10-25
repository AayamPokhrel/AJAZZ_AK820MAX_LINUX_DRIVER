use crate::theme::ThemeColors;
use ajazz_rgb::color::Color;
use ajazz_rgb::layout::{KEYS, TOTAL_LEDS};
use egui::{
    pos2, vec2, Color32, Pos2, Rect, Rounding, Sense, Stroke, Ui,
};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum PresetSelection {
    Wasd,
    Arrows,
    FunctionRow,
    NumberRow,
    Alphas,
    Modifiers,
    All,
}

pub struct KeyboardCanvasState {
    /// Currently selected key names
    pub selected_keys: HashSet<&'static str>,
    /// Per-key custom painted colors (key name -> RGB Color)
    pub custom_colors: [Color; TOTAL_LEDS],
    /// Marquee drag selection box start position
    drag_start: Option<Pos2>,
}

impl Default for KeyboardCanvasState {
    fn default() -> Self {
        Self {
            selected_keys: HashSet::new(),
            custom_colors: [Color::new(0, 242, 254); TOTAL_LEDS],
            drag_start: None,
        }
    }
}

impl KeyboardCanvasState {
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
        for key in KEYS {
            if self.selected_keys.contains(key.name) && key.led_idx < TOTAL_LEDS {
                self.custom_colors[key.led_idx] = color;
            }
        }
    }
}

/// Renders the interactive 75% 82-key AK820 Max visual keyboard canvas.
pub fn render_keyboard_canvas(
    ui: &mut Ui,
    state: &mut KeyboardCanvasState,
    preview_frame: &[Color; TOTAL_LEDS],
    audio_levels: Option<&[f32; 10]>,
    is_custom_mode: bool,
) {
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
            }

            // Determine key illumination color
            let led_color = if is_custom_mode {
                state.custom_colors[key.led_idx]
            } else if let Some(bands) = audio_levels {
                let band = key.audio_band.min(9);
                let level = bands[band];
                let norm_y = (key.rect.center_y() - 40.0) / 180.0;
                let height_from_bottom = (1.0 - norm_y).clamp(0.0, 1.0);
                if height_from_bottom <= level * 1.05 {
                    let grad_col = ThemeColors::EQUALIZER_GRADIENT[band];
                    Color::new(grad_col.r(), grad_col.g(), grad_col.b())
                } else {
                    preview_frame[key.led_idx]
                }
            } else {
                preview_frame[key.led_idx]
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
}
