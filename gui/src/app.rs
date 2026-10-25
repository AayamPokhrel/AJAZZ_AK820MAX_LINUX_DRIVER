use crate::audio_worker::AudioVisualizerWorker;
use crate::ipc::HardwareCoordinator;
use crate::theme::{apply_theme, ThemeColors};
use crate::views::device_status::{render_device_header, render_hardware_settings_panel};
use crate::views::effects_studio::{render_effects_studio, EffectsStudioState, QUICK_SWATCHES};
use crate::views::keyboard_canvas::{render_keyboard_canvas, KeyboardCanvasState};
use crate::views::music_studio::{render_music_studio, MusicStudioState};
use ajazz_rgb::color::Color;
use ajazz_rgb::effects::FrameRenderer;
use ajazz_rgb::layout::TOTAL_LEDS;
use egui::{vec2, Color32, Rounding, Stroke};
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

        let sleep_mins = 15;

        Self {
            current_tab: initial_tab.unwrap_or(AppTab::Effects),
            coordinator,
            audio_worker: AudioVisualizerWorker::new(),
            canvas_state: KeyboardCanvasState::default(),
            effects_state: EffectsStudioState::default(),
            music_state: MusicStudioState::default(),
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

        // Background status refresh every 5 seconds
        if self.last_status_poll.elapsed().as_secs() >= 5 {
            self.last_status_poll = Instant::now();
            self.coordinator.refresh();
        }

        // Render live preview frame based on current effect
        let preview_frame = FrameRenderer::render(
            self.effects_state.current_settings.mode,
            t,
            self.effects_state.current_settings.custom_color,
            self.effects_state.current_settings.brightness as f32 / 5.0,
        );

        // Fetch audio levels if music visualizer is active
        let audio_state = self.audio_worker.state();
        let audio_levels_opt = if audio_state.is_active {
            Some(&audio_state.levels)
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
                                        render_keyboard_canvas(
                                            ui,
                                            &mut self.canvas_state,
                                            &preview_frame,
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
                                                egui::RichText::new("Interactive Per-Key Custom RGB Studio")
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
                                            egui::RichText::new("Click keys or drag a selection box to select keys. Choose a color below to paint selected keys.")
                                                .small()
                                                .color(ThemeColors::TEXT_MUTED),
                                        );

                                        ui.add_space(8.0);

                                        render_keyboard_canvas(
                                            ui,
                                            &mut self.canvas_state,
                                            &preview_frame,
                                            None,
                                            true,
                                        );

                                        ui.add_space(12.0);

                                        // Color Palette Swatches for Painting
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
                                                }
                                            }
                                        });

                                        ui.add_space(12.0);

                                        // Apply Custom Painting to Keyboard
                                        ui.horizontal(|ui| {
                                            let apply_all_btn = egui::Button::new(
                                                egui::RichText::new("⚡ Save Custom Layout to Keyboard")
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

                                            if ui.add(egui::Button::new("Clear All Painting").rounding(Rounding::same(8.0))).clicked() {
                                                self.canvas_state.custom_colors = [Color::new(0, 0, 0); TOTAL_LEDS];
                                            }
                                        });
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
