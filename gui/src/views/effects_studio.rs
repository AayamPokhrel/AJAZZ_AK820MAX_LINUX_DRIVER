use crate::ipc::HardwareCoordinator;
use crate::theme::ThemeColors;
use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{Direction, EffectMode, EffectSettings};
use egui::{vec2, Color32, Rounding, Stroke, Ui};
use std::str::FromStr;
use std::time::SystemTime;

pub struct EffectsStudioState {
    pub current_settings: EffectSettings,
    pub is_custom_color: bool,
    pub hex_input: String,
    pub status_message: Option<(String, bool)>, // (message, is_success)
    rng_seed: u32,
}

impl Default for EffectsStudioState {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros() as u32)
            .unwrap_or(0xFEED_FACE);

        Self {
            current_settings: EffectSettings::default(),
            is_custom_color: false,
            hex_input: "#00F2FE".to_string(),
            status_message: None,
            rng_seed: seed,
        }
    }
}

impl EffectsStudioState {
    pub fn random_color(&mut self) -> Color {
        self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let h = (self.rng_seed >> 8) as f32 / 16777216.0;
        Color::from_hsv(h, 0.95, 1.0)
    }
}

pub const QUICK_SWATCHES: &[(&str, Color)] = &[
    ("Cyan", Color { r: 0, g: 242, b: 254 }),
    ("Purple", Color { r: 168, g: 85, b: 247 }),
    ("Pink", Color { r: 244, g: 63, b: 94 }),
    ("Emerald", Color { r: 16, g: 185, b: 129 }),
    ("Blue", Color { r: 59, g: 130, b: 246 }),
    ("Amber", Color { r: 245, g: 158, b: 11 }),
    ("Crimson", Color { r: 239, g: 68, b: 68 }),
    ("Teal", Color { r: 20, g: 184, b: 166 }),
    ("Indigo", Color { r: 99, g: 102, b: 241 }),
    ("Violet", Color { r: 139, g: 92, b: 246 }),
    ("Rose", Color { r: 244, g: 114, b: 182 }),
    ("Lime", Color { r: 132, g: 204, b: 22 }),
    ("Sky", Color { r: 56, g: 189, b: 248 }),
    ("Orange", Color { r: 249, g: 115, b: 22 }),
    ("White", Color { r: 255, g: 255, b: 255 }),
    ("Ice", Color { r: 200, g: 240, b: 255 }),
];

pub fn render_effects_studio(
    ui: &mut Ui,
    state: &mut EffectsStudioState,
    coordinator: &mut HardwareCoordinator,
) {
    ui.vertical(|ui| {
        // Effects Selector Header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Built-in Lighting Effects Studio")
                    .heading()
                    .color(ThemeColors::TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((msg, ok)) = &state.status_message {
                    let col = if *ok {
                        ThemeColors::STATUS_SUCCESS
                    } else {
                        ThemeColors::STATUS_ERROR
                    };
                    ui.label(egui::RichText::new(msg).color(col).small());
                }
            });
        });

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Click any effect or adjust controls to immediately see changes on the physical keyboard.")
                .small()
                .color(ThemeColors::TEXT_MUTED),
        );

        ui.add_space(8.0);

        // Section 1: Interactive Typing Reactive RGB Modes
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("⌨️ Interactive Typing Reactive Modes")
                    .strong()
                    .color(ThemeColors::ACCENT_CYAN),
            );
            ui.label(
                egui::RichText::new("LIVE STREAMING (30-60 FPS)")
                    .small()
                    .color(ThemeColors::STATUS_SUCCESS),
            );
        });
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Type on your physical keyboard to trigger dynamic reactive effects, shockwaves, thermal heat, and random gradient splashes.")
                .small()
                .color(ThemeColors::TEXT_MUTED),
        );
        ui.add_space(6.0);

        let typing_effects = [
            ("⚡ Gradient Splash", EffectMode::TypingSplash, "Dynamic random gradient waves ripple outwards on typing"),
            ("🎲 Chaos Roulette", EffectMode::TypingRoulette, "Every keypress randomly triggers lasers, ripples & sparks"),
            ("🌊 Neon Shockwave", EffectMode::TypingShockwave, "Radial shockwaves expanding at 500 px/s with interference"),
            ("🔥 Thermal Heatmap", EffectMode::TypingHeatmap, "Physical heat diffusion & glowing ember blackbody ramp"),
            ("🟢 Matrix Code Rain", EffectMode::TypingMatrix, "Digital matrix drops cascading down columns of pressed keys"),
            ("🎯 Crossfire Lasers", EffectMode::TypingLaser, "Horizontal & vertical neon beam intersections"),
            ("✨ Starlight Sparks", EffectMode::TypingSparks, "Explosive particle sparklers bursting with physics"),
            ("🌌 Cosmic Aurora", EffectMode::TypingAurora, "Undulating ribbons swelling with keystroke rhythm"),
        ];

        egui::Grid::new("typing_effects_grid")
            .spacing(vec2(8.0, 8.0))
            .show(ui, |ui| {
                for (idx, (label, mode, tip)) in typing_effects.iter().enumerate() {
                    let is_active = state.current_settings.mode == *mode;
                    let btn = egui::Button::new(*label)
                        .min_size(vec2(165.0, 38.0))
                        .rounding(Rounding::same(8.0))
                        .fill(if is_active {
                            Color32::from_rgb(45, 20, 75)
                        } else {
                            ThemeColors::BG_CARD
                        })
                        .stroke(if is_active {
                            Stroke::new(2.0_f32, ThemeColors::ACCENT_CYAN)
                        } else {
                            Stroke::new(1.0_f32, Color32::from_rgb(60, 45, 90))
                        });

                    let resp = ui.add(btn).on_hover_text(*tip);
                    if resp.clicked() {
                        state.current_settings.mode = *mode;
                        let _ = coordinator.set_effect(&state.current_settings);
                        state.status_message = Some((
                            format!("Activated interactive {}", label),
                            true,
                        ));
                    }

                    if (idx + 1) % 4 == 0 {
                        ui.end_row();
                    }
                }
            });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        // Section 2: Hardware Built-in Effects (16 modes from kb-k600t.xml)
        ui.label(
            egui::RichText::new("⚙️ Hardware Built-in Firmware Effects")
                .strong()
                .color(ThemeColors::TEXT_SECONDARY),
        );
        ui.add_space(6.0);

        let hw_effects = [
            ("Rainbow Spectrum", EffectMode::Spectrum),
            ("Static Constant", EffectMode::Static),
            ("Overall Breathing", EffectMode::Breathing),
            ("Windmill", EffectMode::Windmill),
            ("Sine Wave", EffectMode::SineWave),
            ("Waterfall", EffectMode::TopBottom),
            ("Storm", EffectMode::Galloping),
            ("Raindrops", EffectMode::Rain),
            ("Ripple L-R", EffectMode::RippleLr),
            ("Ripple U-D", EffectMode::RippleUd),
            ("Cross Blossom", EffectMode::Cross),
            ("Key Reactive", EffectMode::KeyBreathing),
            ("Stars Twinkle", EffectMode::Starry),
            ("Flash Away", EffectMode::Ejection),
            ("Kaleidoscope", EffectMode::Kaleidoscope),
            ("Picture Scroll", EffectMode::Scroll),
            ("Lights Off", EffectMode::Off),
        ];

        egui::Grid::new("hw_effects_grid")
            .spacing(vec2(8.0, 8.0))
            .show(ui, |ui| {
                for (idx, (label, mode)) in hw_effects.iter().enumerate() {
                    let is_active = state.current_settings.mode == *mode;
                    let btn = egui::Button::new(*label)
                        .min_size(vec2(130.0, 34.0))
                        .rounding(Rounding::same(8.0))
                        .fill(if is_active {
                            ThemeColors::ACCENT_PURPLE
                        } else {
                            ThemeColors::BG_CARD
                        })
                        .stroke(if is_active {
                            Stroke::new(1.8_f32, ThemeColors::ACCENT_CYAN)
                        } else {
                            Stroke::new(1.0_f32, ThemeColors::BORDER_MUTED)
                        });

                    if ui.add(btn).clicked() {
                        state.current_settings.mode = *mode;
                        let _ = coordinator.set_effect(&state.current_settings);
                        state.status_message = Some((
                            format!("Activated hardware {}", label),
                            true,
                        ));
                    }

                    if (idx + 1) % 5 == 0 {
                        ui.end_row();
                    }
                }
            });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        // Responsive Controls Panel
        ui.columns(2, |columns| {
            // Left Column: Parameters
            columns[0].vertical(|ui| {
                ui.label(
                    egui::RichText::new("Effect Tuning & Dynamics")
                        .strong()
                        .color(ThemeColors::TEXT_SECONDARY),
                );
                ui.add_space(8.0);

                // Brightness Slider (1..=5)
                ui.horizontal(|ui| {
                    ui.label("Brightness:");
                    let resp = ui.add(
                        egui::Slider::new(&mut state.current_settings.brightness, 1..=5)
                            .show_value(true)
                            .text("levels"),
                    );
                    if resp.changed() {
                        let _ = coordinator.set_effect(&state.current_settings);
                    }
                });

                // Speed Slider (1..=5)
                ui.horizontal(|ui| {
                    ui.label("Speed:       ");
                    let resp = ui.add(
                        egui::Slider::new(&mut state.current_settings.speed, 1..=5)
                            .show_value(true)
                            .text("levels"),
                    );
                    if resp.changed() {
                        let _ = coordinator.set_effect(&state.current_settings);
                    }
                });

                // Direction Toggle
                ui.horizontal(|ui| {
                    ui.label("Direction:  ");
                    let is_right = state.current_settings.direction == Direction::Right;
                    if ui
                        .selectable_label(is_right, "Right →")
                        .clicked()
                    {
                        state.current_settings.direction = Direction::Right;
                        let _ = coordinator.set_effect(&state.current_settings);
                    }
                    if ui
                        .selectable_label(!is_right, "← Left")
                        .clicked()
                    {
                        state.current_settings.direction = Direction::Left;
                        let _ = coordinator.set_effect(&state.current_settings);
                    }
                });

                ui.add_space(12.0);

                // Apply button
                let apply_btn = egui::Button::new(
                    egui::RichText::new("⚡ Re-apply Effect to Hardware")
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(ThemeColors::ACCENT_CYAN)
                .min_size(vec2(220.0, 36.0))
                .rounding(Rounding::same(8.0));

                if ui.add(apply_btn).clicked() {
                    match coordinator.set_effect(&state.current_settings) {
                        Ok(_) => {
                            state.status_message = Some((
                                format!("Applied {} successfully!", state.current_settings.mode.name()),
                                true,
                            ));
                        }
                        Err(e) => {
                            state.status_message = Some((format!("Error: {}", e), false));
                        }
                    }
                }
            });

            // Right Column: Color Selection & Palette
            columns[1].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Color Selection & Generators")
                            .strong()
                            .color(ThemeColors::TEXT_SECONDARY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("🎲 Random Color")
                                    .fill(ThemeColors::BG_CARD)
                                    .rounding(Rounding::same(6.0)),
                            )
                            .clicked()
                        {
                            let c = state.random_color();
                            state.is_custom_color = true;
                            state.current_settings.custom_color = Some(c);
                            state.hex_input = c.to_hex();
                            let _ = coordinator.set_effect(&state.current_settings);
                            state.status_message = Some((format!("Random color {} applied", c.to_hex()), true));
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(!state.is_custom_color, "🌈 Rainbow Flow")
                        .clicked()
                    {
                        state.is_custom_color = false;
                        state.current_settings.palette_color = PaletteColor::Rainbow;
                        state.current_settings.custom_color = None;
                        let _ = coordinator.set_effect(&state.current_settings);
                    }

                    if ui
                        .selectable_label(state.is_custom_color, "🎨 Custom Color")
                        .clicked()
                    {
                        state.is_custom_color = true;
                        if state.current_settings.custom_color.is_none() {
                            state.current_settings.custom_color = Some(Color::new(0, 242, 254));
                        }
                        let _ = coordinator.set_effect(&state.current_settings);
                    }
                });

                ui.add_space(8.0);

                // Palette Swatches
                ui.label(
                    egui::RichText::new("Curated Swatches:")
                        .small()
                        .color(ThemeColors::TEXT_MUTED),
                );

                ui.horizontal_wrapped(|ui| {
                    for (_name, col) in QUICK_SWATCHES {
                        let c32 = Color32::from_rgb(col.r, col.g, col.b);
                        let is_current = state
                            .current_settings
                            .custom_color
                            .map_or(false, |c| c == *col);

                        let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
                        ui.painter().rect_filled(rect, Rounding::same(6.0), c32);

                        let stroke = if is_current {
                            Stroke::new(2.0_f32, Color32::WHITE)
                        } else {
                            Stroke::new(1.0_f32, ThemeColors::BORDER_MUTED)
                        };
                        ui.painter().rect_stroke(rect, Rounding::same(6.0), stroke);

                        if resp.clicked() {
                            state.is_custom_color = true;
                            state.current_settings.custom_color = Some(*col);
                            state.hex_input = col.to_hex();
                            let _ = coordinator.set_effect(&state.current_settings);
                        }
                    }
                });

                ui.add_space(8.0);

                // Hex input
                ui.horizontal(|ui| {
                    ui.label("Hex Color:");
                    let text_edit = ui.add(
                        egui::TextEdit::singleline(&mut state.hex_input)
                            .desired_width(90.0)
                            .font(egui::TextStyle::Monospace),
                    );

                    if text_edit.lost_focus() || text_edit.changed() {
                        if let Ok(parsed) = Color::from_str(&state.hex_input) {
                            state.is_custom_color = true;
                            state.current_settings.custom_color = Some(parsed);
                            let _ = coordinator.set_effect(&state.current_settings);
                        }
                    }

                    if let Some(col) = state.current_settings.custom_color {
                        let c32 = Color32::from_rgb(col.r, col.g, col.b);
                        let (rect, _) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, Rounding::same(6.0), c32);
                        ui.painter().rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, Color32::WHITE));
                    }
                });
            });
        });
    });
}
