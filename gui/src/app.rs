use crate::audio_worker::AudioVisualizerWorker;
use crate::ipc::HardwareCoordinator;
use crate::theme::{apply_theme, ThemeColors};
use crate::views::device_status::{render_device_header, render_hardware_settings_panel};
use crate::views::effects_studio::{render_effects_studio, EffectsStudioState, QUICK_SWATCHES};
use crate::views::keyboard_canvas::{
    render_keyboard_canvas, GradientDirection, KeyboardCanvasState, PresetTheme,
};
use crate::views::music_studio::{render_music_studio, MusicStudioState};
use ajazz_audio::capture::AudioSource;
use ajazz_audio::visualizer::{ColorMode, ColorPalette, VisualizerStyle};
use ajazz_rgb::color::Color;
use ajazz_rgb::effects::{EffectSettings, FrameRenderer};
use ajazz_rgb::layout::TOTAL_LEDS;
use ajazz_rgb::storage::PersistentConfig;
use egui::{vec2, Color32, Rounding, Stroke};
use std::str::FromStr;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    CustomCanvas,
    Effects,
    Music,
    Hardware,
}

pub struct Ak820App {
    pub current_tab: AppTab,
    pub coordinator: HardwareCoordinator,
    pub audio_worker: AudioVisualizerWorker,
    pub canvas_state: KeyboardCanvasState,
    pub effects_state: EffectsStudioState,
    pub music_state: MusicStudioState,
    pub sleep_minutes: u8,
    pub status_message: Option<(String, bool)>,
    start_instant: Instant,
    last_status_poll: Instant,
}

impl Ak820App {
    pub fn new(_cc: &eframe::CreationContext, initial_tab: Option<AppTab>) -> Self {
        let mut coordinator = HardwareCoordinator::new();
        coordinator.refresh();

        let saved_cfg = PersistentConfig::load();
        let current_tab = initial_tab.unwrap_or_else(|| match saved_cfg.active_category.as_str() {
            "canvas" => AppTab::CustomCanvas,
            "music" => AppTab::Music,
            _ => AppTab::Effects,
        });

        let mut effects_state = EffectsStudioState::default();
        effects_state.current_settings = EffectSettings::from(saved_cfg.lighting.clone());
        effects_state.is_custom_color = saved_cfg.lighting.custom_color.is_some();
        if let Some(c) = saved_cfg.lighting.custom_color {
            effects_state.hex_input = c.to_hex();
        }

        let mut canvas_state = KeyboardCanvasState::default();
        for (i, hex) in saved_cfg.canvas.colors.iter().enumerate() {
            if i < TOTAL_LEDS {
                if let Ok(c) = Color::from_str(hex) {
                    canvas_state.custom_colors[i] = c;
                }
            }
        }

        let mut music_state = MusicStudioState::default();
        music_state.style = VisualizerStyle::from_name(&saved_cfg.music.style)
            .unwrap_or(VisualizerStyle::Spectrum);
        music_state.color_mode = ColorMode::from_name(&saved_cfg.music.color_mode)
            .unwrap_or(ColorMode::Palette(ColorPalette::StockClassic));
        music_state.sensitivity = saved_cfg.music.sensitivity;
        music_state.noise_gate = saved_cfg.music.noise_gate;
        music_state.decay_speed = saved_cfg.music.decay_speed;
        music_state.peak_hold = saved_cfg.music.peak_hold;
        music_state.fps = saved_cfg.music.target_fps;
        music_state.morph_speed = saved_cfg.music.morph_speed;
        music_state.audio_source = match saved_cfg.music.audio_source.as_str() {
            "Microphone" => AudioSource::Microphone,
            _ => AudioSource::SystemLoopback,
        };

        let sleep_mins = saved_cfg.device.sleep_minutes;

        Self {
            current_tab,
            coordinator,
            audio_worker: AudioVisualizerWorker::new(),
            canvas_state,
            effects_state,
            music_state,
            sleep_minutes: sleep_mins,
            status_message: None,
            start_instant: Instant::now(),
            last_status_poll: Instant::now(),
        }
    }
}

impl eframe::App for Ak820App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply centralized theme
        apply_theme(ctx);

        let t = self.start_instant.elapsed().as_secs_f32();

        // Background status refresh every 5 seconds (paused during active music visualization to avoid USB contention)
        if !self.audio_worker.is_active() && self.last_status_poll.elapsed().as_secs() >= 5 {
            self.last_status_poll = Instant::now();
            self.coordinator.refresh();
        }
        // Capture GUI key events to animate preview interactively
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key, pressed: true, .. } = event {
                    let key_str = format!("{:?}", key).to_lowercase();
                    if let Some(def) = ajazz_rgb::layout::find_key_by_name(&key_str) {
                        FrameRenderer::on_preview_key_press(
                            def.hw_idx,
                            def.row,
                            def.col,
                            def.rect.center_x(),
                            def.rect.center_y(),
                        );
                    }
                }
            }
        });

        // Render live preview frame based on current effect
        let preview_frame = FrameRenderer::render(
            self.effects_state.current_settings.mode,
            t,
            self.effects_state.current_settings.custom_color,
            self.effects_state.current_settings.brightness as f32 / 5.0,
        );

        // Fetch audio levels if music visualizer is active
        let audio_state = self.audio_worker.state();
        let audio_levels_opt: Option<&[f32]> = if audio_state.is_active {
            Some(audio_state.levels.as_slice())
        } else {
            None
        };

        // Smooth animations request continuous repaints
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Top Header
        egui::TopBottomPanel::top("top_header")
            .frame(egui::Frame::none().fill(ThemeColors::BG_MAIN).inner_margin(12.0))
            .show(ctx, |ui| {
                render_device_header(ui, &mut self.coordinator);
                ui.add_space(8.0);

                // Tab Selector Bar
                ui.horizontal(|ui| {
                    let tabs = [
                        (AppTab::Effects, "✨ Lighting Effects Studio"),
                        (AppTab::Music, "🎵 Audio-Reactive Visualizer"),
                        (AppTab::CustomCanvas, "🎹 75% Visual Canvas & Custom Paint"),
                        (AppTab::Hardware, "⚙️ Device & Power Settings"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.current_tab == tab;
                        let btn = egui::Button::new(label)
                            .min_size(vec2(140.0, 32.0))
                            .rounding(Rounding::same(8.0))
                            .fill(if is_active {
                                ThemeColors::BG_CARD
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(if is_active {
                                Stroke::new(1.5_f32, ThemeColors::ACCENT_CYAN)
                            } else {
                                Stroke::NONE
                            });

                        if ui.add(btn).clicked() {
                            self.current_tab = tab;
                            match tab {
                                AppTab::CustomCanvas => {
                                    if self.canvas_state.live_mirror {
                                        self.coordinator.update_custom_live(
                                            &self.canvas_state.custom_colors,
                                            5,
                                        );
                                    }
                                }
                                AppTab::Effects => {
                                    let _ = self
                                        .coordinator
                                        .set_effect(&self.effects_state.current_settings);
                                }
                                _ => {}
                            }
                        }
                    }
                });
            });

        // Main Body Content
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ThemeColors::BG_PANEL).inner_margin(16.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.current_tab {
                            AppTab::Effects => {
                                // Live Visual Canvas Preview
                                ui.group(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Live Keyboard Matrix Preview")
                                                    .strong()
                                                    .color(ThemeColors::TEXT_SECONDARY),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("Mode: {}", self.effects_state.current_settings.mode.name()))
                                                        .small()
                                                        .color(ThemeColors::ACCENT_CYAN),
                                                );
                                            });
                                        });
                                        ui.add_space(6.0);
                                        render_keyboard_canvas(
                                            ui,
                                            &mut self.canvas_state,
                                            &preview_frame,
                                            None,
                                            false,
                                        );
                                    });
                                });

                                ui.add_space(14.0);
                                render_effects_studio(
                                    ui,
                                    &mut self.effects_state,
                                    &mut self.coordinator,
                                );
                            }

                            AppTab::Music => {
                                // Live Visual Canvas with Audio Reaction
                                ui.group(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Audio-Reactive Keycap Visualizer Preview")
                                                    .strong()
                                                    .color(ThemeColors::TEXT_SECONDARY),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("Style: {}", self.music_state.style.name()))
                                                        .small()
                                                        .color(ThemeColors::ACCENT_CYAN),
                                                );
                                            });
                                        });
                                        ui.add_space(6.0);
                                        let live_music_frame = if self.audio_worker.is_active() {
                                            self.audio_worker.state().frame
                                        } else {
                                            preview_frame
                                        };
                                        render_keyboard_canvas(
                                            ui,
                                            &mut self.canvas_state,
                                            &live_music_frame,
                                            audio_levels_opt,
                                            false,
                                        );
                                    });
                                });

                                ui.add_space(14.0);
                                render_music_studio(
                                    ui,
                                    &mut self.music_state,
                                    &mut self.coordinator,
                                    &mut self.audio_worker,
                                );
                            }

                            AppTab::CustomCanvas => {
                                ui.group(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("75% Visual Canvas & Custom Per-Key RGB Studio")
                                                    .heading()
                                                    .color(ThemeColors::TEXT_PRIMARY),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if let Some((msg, ok)) = &self.status_message {
                                                    let col = if *ok { ThemeColors::STATUS_SUCCESS } else { ThemeColors::STATUS_ERROR };
                                                    ui.label(egui::RichText::new(msg).color(col).small());
                                                }
                                            });
                                        });

                                        ui.label(
                                            egui::RichText::new("Click keys or drag marquee to select keys. Use the multi-stop gradient generator, randomizer, or color palettes below.")
                                                .small()
                                                .color(ThemeColors::TEXT_MUTED),
                                        );

                                        ui.add_space(8.0);

                                        let canvas_changed = render_keyboard_canvas(
                                            ui,
                                            &mut self.canvas_state,
                                            &preview_frame,
                                            None,
                                            true,
                                        );

                                        let mut trigger_hardware_mirror = canvas_changed && self.canvas_state.live_mirror;

                                        ui.add_space(12.0);

                                        // Section 1: Color Generators & Randomizers Toolbar
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                egui::RichText::new("Generators:")
                                                    .strong()
                                                    .color(ThemeColors::TEXT_SECONDARY),
                                            );

                                            if ui
                                                .add(egui::Button::new("🎲 Randomize All Keys").fill(ThemeColors::BG_CARD).rounding(Rounding::same(6.0)))
                                                .clicked()
                                            {
                                                self.canvas_state.randomize_all();
                                                trigger_hardware_mirror = true;
                                            }

                                            let sel_count = self.canvas_state.selected_keys.len();
                                            let sel_text = if sel_count > 0 {
                                                format!("🎲 Randomize Selected ({})", sel_count)
                                            } else {
                                                "🎲 Randomize Selected".to_string()
                                            };
                                            if ui
                                                .add(egui::Button::new(sel_text).fill(ThemeColors::BG_CARD).rounding(Rounding::same(6.0)))
                                                .clicked()
                                            {
                                                self.canvas_state.randomize_selected();
                                                trigger_hardware_mirror = true;
                                            }

                                            if ui
                                                .add(egui::Button::new("🎲 Random Gradient").fill(ThemeColors::BG_CARD).rounding(Rounding::same(6.0)))
                                                .clicked()
                                            {
                                                self.canvas_state.randomize_gradient(self.canvas_state.gradient_selected_only);
                                                trigger_hardware_mirror = true;
                                            }

                                            ui.separator();

                                            if ui.checkbox(&mut self.canvas_state.live_mirror, "⚡ Live Hardware Mirror (Instant)").changed() {
                                                if self.canvas_state.live_mirror {
                                                    trigger_hardware_mirror = true;
                                                }
                                            }
                                        });

                                        ui.add_space(10.0);

                                        // Section 2: Multi-Stop Spatial Gradient Generator
                                        ui.group(|ui| {
                                            ui.vertical(|ui| {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("Multi-Stop Spatial Gradient Generator:")
                                                            .strong()
                                                            .color(ThemeColors::TEXT_PRIMARY),
                                                    );

                                                    ui.separator();
                                                    ui.label("Direction:");

                                                    let dirs = [
                                                        (GradientDirection::Horizontal, "Horizontal ↔"),
                                                        (GradientDirection::Vertical, "Vertical ↕"),
                                                        (GradientDirection::Radial, "Radial ⊙"),
                                                        (GradientDirection::Diagonal, "Diagonal ⤢"),
                                                    ];
                                                    for (d, label) in dirs {
                                                        if ui.selectable_label(self.canvas_state.gradient_dir == d, label).clicked() {
                                                            self.canvas_state.gradient_dir = d;
                                                        }
                                                    }
                                                });

                                                ui.add_space(6.0);

                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label("Stop 1:");
                                                    let mut c1_32 = Color32::from_rgb(
                                                        self.canvas_state.gradient_c1.r,
                                                        self.canvas_state.gradient_c1.g,
                                                        self.canvas_state.gradient_c1.b,
                                                    );
                                                    if ui.color_edit_button_srgba(&mut c1_32).changed() {
                                                        self.canvas_state.gradient_c1 = Color::new(c1_32.r(), c1_32.g(), c1_32.b());
                                                    }

                                                    ui.label("Stop 2:");
                                                    let mut c2_32 = Color32::from_rgb(
                                                        self.canvas_state.gradient_c2.r,
                                                        self.canvas_state.gradient_c2.g,
                                                        self.canvas_state.gradient_c2.b,
                                                    );
                                                    if ui.color_edit_button_srgba(&mut c2_32).changed() {
                                                        self.canvas_state.gradient_c2 = Color::new(c2_32.r(), c2_32.g(), c2_32.b());
                                                    }

                                                    let mut has_c3 = self.canvas_state.gradient_c3.is_some();
                                                    if ui.checkbox(&mut has_c3, "3rd Stop:").changed() {
                                                        if has_c3 {
                                                            self.canvas_state.gradient_c3 = Some(Color::new(255, 215, 0));
                                                        } else {
                                                            self.canvas_state.gradient_c3 = None;
                                                        }
                                                    }

                                                    if let Some(ref mut c3) = self.canvas_state.gradient_c3 {
                                                        let mut c3_32 = Color32::from_rgb(c3.r, c3.g, c3.b);
                                                        if ui.color_edit_button_srgba(&mut c3_32).changed() {
                                                            *c3 = Color::new(c3_32.r(), c3_32.g(), c3_32.b());
                                                        }
                                                    }

                                                    ui.separator();

                                                    let apply_all = ui.add(
                                                        egui::Button::new("✨ Apply (All Keys)")
                                                            .fill(ThemeColors::ACCENT_PURPLE)
                                                            .rounding(Rounding::same(6.0)),
                                                    );
                                                    if apply_all.clicked() {
                                                        self.canvas_state.apply_spatial_gradient(
                                                            self.canvas_state.gradient_c1,
                                                            self.canvas_state.gradient_c2,
                                                            self.canvas_state.gradient_c3,
                                                            self.canvas_state.gradient_dir,
                                                            false,
                                                        );
                                                        trigger_hardware_mirror = true;
                                                    }

                                                    let apply_sel = ui.add(
                                                        egui::Button::new("✨ Apply (Selected Keys)")
                                                            .fill(ThemeColors::BG_CARD)
                                                            .rounding(Rounding::same(6.0)),
                                                    );
                                                    if apply_sel.clicked() {
                                                        self.canvas_state.apply_spatial_gradient(
                                                            self.canvas_state.gradient_c1,
                                                            self.canvas_state.gradient_c2,
                                                            self.canvas_state.gradient_c3,
                                                            self.canvas_state.gradient_dir,
                                                            true,
                                                        );
                                                        trigger_hardware_mirror = true;
                                                    }
                                                });
                                            });
                                        });

                                        ui.add_space(8.0);

                                        // Section 3: Curated Aesthetic Theme Presets
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                egui::RichText::new("Preset Themes:")
                                                    .strong()
                                                    .color(ThemeColors::TEXT_SECONDARY),
                                            );

                                            let themes = [
                                                (PresetTheme::Cyberpunk, "🌆 Cyberpunk"),
                                                (PresetTheme::Synthwave, "🌴 Synthwave"),
                                                (PresetTheme::ThermalFlame, "🔥 Thermal Flame"),
                                                (PresetTheme::OceanicIce, "❄️ Oceanic Ice"),
                                                (PresetTheme::MatrixTerminal, "🟢 Matrix Terminal"),
                                                (PresetTheme::GamerWasd, "🎮 Gamer WASD"),
                                            ];

                                            for (theme, name) in themes {
                                                if ui.add(egui::Button::new(name).fill(ThemeColors::BG_CARD).rounding(Rounding::same(6.0))).clicked() {
                                                    self.canvas_state.apply_preset_theme(theme);
                                                    trigger_hardware_mirror = true;
                                                }
                                            }
                                        });

                                        ui.add_space(8.0);

                                        // Section 4: Quick Swatches for manual painting
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Paint Palette:")
                                                    .strong()
                                                    .color(ThemeColors::TEXT_SECONDARY),
                                            );

                                            for (name, col) in QUICK_SWATCHES {
                                                let c32 = Color32::from_rgb(col.r, col.g, col.b);
                                                let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), egui::Sense::click());
                                                ui.painter().rect_filled(rect, Rounding::same(6.0), c32);
                                                ui.painter().rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, ThemeColors::BORDER_MUTED));

                                                if resp.on_hover_text(*name).clicked() {
                                                    self.canvas_state.paint_selected(*col);
                                                    trigger_hardware_mirror = true;
                                                }
                                            }
                                        });

                                        ui.add_space(12.0);

                                        // Section 5: Save & Commit
                                        ui.horizontal(|ui| {
                                            let apply_all_btn = egui::Button::new(
                                                egui::RichText::new("💾 Save Custom Layout to Keyboard")
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(ThemeColors::ACCENT_CYAN)
                                            .min_size(vec2(240.0, 36.0))
                                            .rounding(Rounding::same(8.0));

                                            if ui.add(apply_all_btn).clicked() {
                                                match self.coordinator.upload_custom_matrix(&self.canvas_state.custom_colors, 5) {
                                                    Ok(_) => {
                                                        self.status_message = Some((
                                                            "Successfully uploaded custom RGB canvas to keyboard!".to_string(),
                                                            true,
                                                        ));
                                                    }
                                                    Err(e) => {
                                                        self.status_message = Some((format!("Upload error: {}", e), false));
                                                    }
                                                }
                                            }

                                            if ui.add(egui::Button::new("🗑️ Clear All Keys").rounding(Rounding::same(8.0))).clicked() {
                                                self.canvas_state.custom_colors = [Color::new(0, 0, 0); TOTAL_LEDS];
                                                trigger_hardware_mirror = true;
                                            }
                                        });

                                        // If real-time mirror triggered, upload frame immediately without blocking
                                        if trigger_hardware_mirror && self.canvas_state.live_mirror {
                                            self.coordinator.update_custom_live(
                                                &self.canvas_state.custom_colors,
                                                5,
                                            );
                                        }
                                    });
                                });
                            }

                            AppTab::Hardware => {
                                render_hardware_settings_panel(
                                    ui,
                                    &mut self.coordinator,
                                    &mut self.sleep_minutes,
                                    &mut self.status_message,
                                );
                            }
                        }
                    });
            });
    }
}
