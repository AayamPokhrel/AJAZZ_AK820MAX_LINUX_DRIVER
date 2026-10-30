use crate::audio_worker::AudioVisualizerWorker;
use crate::ipc::HardwareCoordinator;
use crate::theme::ThemeColors;
use ajazz_audio::capture::AudioSource;
use ajazz_audio::visualizer::{ColorPalette, VisualizerStyle};
use ajazz_rgb::color::Color;
use egui::{pos2, vec2, Color32, Rect, Rounding, Stroke, Ui};

pub struct MusicStudioState {
    pub style: VisualizerStyle,
    pub palette: ColorPalette,
    pub sensitivity: f32,
    pub noise_gate: f32,
    pub fps: u32,
    pub audio_source: AudioSource,
    pub custom_tint: Option<Color>,
    pub custom_peak: Option<Color>,
    pub status_message: Option<(String, bool)>,
}

impl Default for MusicStudioState {
    fn default() -> Self {
        Self {
            style: VisualizerStyle::Spectrum,
            palette: ColorPalette::StockClassic,
            sensitivity: 1.2,
            noise_gate: 0.05,
            fps: 60,
            audio_source: AudioSource::SystemLoopback,
            custom_tint: None,
            custom_peak: None,
            status_message: None,
        }
    }
}

pub fn render_music_studio(
    ui: &mut Ui,
    state: &mut MusicStudioState,
    coordinator: &mut HardwareCoordinator,
    audio_worker: &mut AudioVisualizerWorker,
) {
    let audio_state = audio_worker.state();
    let is_active = audio_worker.is_active();

    ui.vertical(|ui| {
        // Music Studio Header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Audio-Reactive Music Studio")
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

        ui.add_space(8.0);

        // Visualizer Style Switcher
        let styles = [
            ("Equalizer Spectrum", VisualizerStyle::Spectrum, "10-band rising equalizer bars"),
            ("Bass Pulse", VisualizerStyle::Pulse, "Dynamic energy pulsation from center"),
            ("Ripple Wave", VisualizerStyle::Wave, "Expanding acoustic radial ripples"),
            ("Matrix Rain", VisualizerStyle::Matrix, "Cascading acoustic matrix rain drops"),
            ("Passion Rock", VisualizerStyle::Passion, "High-intensity transient bursts"),
        ];

        ui.horizontal_wrapped(|ui| {
            for (label, style, _tooltip) in &styles {
                let is_selected = state.style == *style;
                let btn = egui::Button::new(*label)
                    .min_size(vec2(130.0, 32.0))
                    .rounding(Rounding::same(8.0))
                    .fill(if is_selected {
                        ThemeColors::ACCENT_CYAN
                    } else {
                        ThemeColors::BG_CARD
                    })
                    .stroke(if is_selected {
                        Stroke::new(1.5_f32, Color32::WHITE)
                    } else {
                        Stroke::new(1.0_f32, ThemeColors::BORDER_MUTED)
                    });

                if ui.add(btn).clicked() {
                    state.style = *style;
                    if is_active {
                        audio_worker.update_config(
                            state.style,
                            state.palette.clone(),
                            state.custom_tint,
                            state.sensitivity,
                            state.noise_gate,
                            state.fps,
                        );
                        let source_str = match &state.audio_source {
                            AudioSource::Microphone => "mic",
                            AudioSource::SystemLoopback => "system",
                            AudioSource::Custom(s) => s.as_str(),
                        };
                        let _ = coordinator.start_music(
                            state.style.name(),
                            state.custom_tint,
                            Some(state.palette.name()),
                            state.sensitivity,
                            Some(state.noise_gate),
                            Some(source_str),
                            state.fps,
                        );
                    }
                }
            }
        });

        ui.add_space(14.0);

        // Real-time 10-Band Frequency Spectrum Analyzer Canvas with Peak Hold
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Real-Time 10-Band Spectrum Analyzer (40 Hz – 16 kHz)")
                            .strong()
                            .color(ThemeColors::TEXT_SECONDARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let status_badge = if is_active {
                            let src_label = match &state.audio_source {
                                AudioSource::SystemLoopback => "DESKTOP AUDIO LOOPBACK",
                                AudioSource::Microphone => "MICROPHONE",
                                AudioSource::Custom(s) => s.as_str(),
                            };
                            egui::RichText::new(format!("● LIVE [{}]", src_label))
                                .color(ThemeColors::STATUS_SUCCESS)
                                .small()
                        } else {
                            egui::RichText::new("○ STANDBY")
                                .color(ThemeColors::TEXT_MUTED)
                                .small()
                        };
                        ui.label(status_badge);
                    });
                });

                ui.add_space(6.0);

                let available_w = ui.available_width().max(320.0);
                let analyzer_h = 130.0f32;
                let (rect, _) = ui.allocate_exact_size(vec2(available_w, analyzer_h), egui::Sense::hover());
                let painter = ui.painter();

                // Background container
                painter.rect_filled(rect, Rounding::same(10.0), ThemeColors::BG_MAIN);
                painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0_f32, ThemeColors::BORDER_SUBTLE));

                let band_labels = ["40Hz", "80Hz", "160Hz", "320Hz", "640Hz", "1.2k", "2.5k", "5kHz", "10k", "16k"];
                let band_count = 10;
                let padding_x = 16.0f32;
                let bottom_padding_y = 24.0f32;
                let top_padding_y = 12.0f32;

                let content_w = available_w - padding_x * 2.0;
                let max_bar_h = analyzer_h - bottom_padding_y - top_padding_y;
                let bar_width = (content_w / (band_count as f32) - 8.0).max(12.0);
                let gap = (content_w - bar_width * (band_count as f32)) / ((band_count - 1) as f32);

                for i in 0..band_count {
                    let level = audio_state.levels[i].clamp(0.0, 1.0);
                    let peak = audio_state.peaks[i].clamp(0.0, 1.0);
                    let bar_color = ThemeColors::EQUALIZER_GRADIENT[i];

                    let bx = rect.min.x + padding_x + (i as f32) * (bar_width + gap);
                    let by_bottom = rect.max.y - bottom_padding_y;
                    let current_bar_h = (level * max_bar_h).max(3.0);
                    let by_top = by_bottom - current_bar_h;

                    let bar_rect = Rect::from_min_max(pos2(bx, by_top), pos2(bx + bar_width, by_bottom));
                    let rounding = Rounding::same(4.0);

                    // Bar fill
                    painter.rect_filled(bar_rect, rounding, bar_color);

                    // Peak decay indicator tick
                    let peak_y = by_bottom - (peak * max_bar_h).max(3.0);
                    let peak_rect = Rect::from_min_max(pos2(bx, peak_y - 2.0), pos2(bx + bar_width, peak_y));
                    painter.rect_filled(peak_rect, Rounding::same(1.0), Color32::WHITE);

                    // Frequency label
                    let label_pos = pos2(bx + bar_width * 0.5, rect.max.y - 12.0);
                    painter.text(
                        label_pos,
                        egui::Align2::CENTER_CENTER,
                        band_labels[i],
                        egui::FontId::monospace(9.0),
                        ThemeColors::TEXT_MUTED,
                    );
                }
            });
        });

        ui.add_space(14.0);

        // Music Controls & Sliders
        ui.columns(2, |columns| {
            // Left Column: Main Action, Audio Source & DSP Tuning
            columns[0].vertical(|ui| {
                ui.label(
                    egui::RichText::new("Visualizer Engine Controls")
                        .strong()
                        .color(ThemeColors::TEXT_SECONDARY),
                );
                ui.add_space(6.0);

                // Start / Stop Master Toggle
                let toggle_btn = if is_active {
                    egui::Button::new(
                        egui::RichText::new("⏹ Stop Music Visualizer")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(ThemeColors::STATUS_ERROR)
                    .min_size(vec2(220.0, 38.0))
                    .rounding(Rounding::same(8.0))
                } else {
                    egui::Button::new(
                        egui::RichText::new("▶ Start Music Visualizer")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(ThemeColors::ACCENT_CYAN)
                    .min_size(vec2(220.0, 38.0))
                    .rounding(Rounding::same(8.0))
                };

                if ui.add(toggle_btn).clicked() {
                    if is_active {
                        audio_worker.stop();
                        let _ = coordinator.stop_music();
                        state.status_message = Some(("Music visualizer stopped.".to_string(), true));
                    } else {
                        audio_worker.start(
                            state.style,
                            state.palette.clone(),
                            state.custom_tint,
                            state.sensitivity,
                            state.noise_gate,
                            state.fps,
                            Some(state.audio_source.clone()),
                            coordinator.direct_device(),
                        );
                        let source_str = match &state.audio_source {
                            AudioSource::Microphone => "mic",
                            AudioSource::SystemLoopback => "system",
                            AudioSource::Custom(s) => s.as_str(),
                        };
                        match coordinator.start_music(
                            state.style.name(),
                            state.custom_tint,
                            Some(state.palette.name()),
                            state.sensitivity,
                            Some(state.noise_gate),
                            Some(source_str),
                            state.fps,
                        ) {
                            Ok(_) => {
                                state.status_message = Some((
                                    format!("Hardware music sync active ({} mode)", state.style.name()),
                                    true,
                                ));
                            }
                            Err(e) => {
                                state.status_message = Some((format!("Direct audio streaming active: {}", e), true));
                            }
                        }
                    }
                }

                ui.add_space(10.0);

                // Audio Source Selector
                ui.label(
                    egui::RichText::new("Audio Input Source")
                        .small()
                        .color(ThemeColors::TEXT_MUTED),
                );
                ui.horizontal(|ui| {
                    let is_system = state.audio_source == AudioSource::SystemLoopback;
                    if ui.selectable_label(is_system, "🔊 System Desktop Audio (Loopback)").clicked() {
                        state.audio_source = AudioSource::SystemLoopback;
                        if is_active {
                            audio_worker.stop();
                            audio_worker.start(
                                state.style,
                                state.palette.clone(),
                                state.custom_tint,
                                state.sensitivity,
                                state.noise_gate,
                                state.fps,
                                Some(state.audio_source.clone()),
                                coordinator.direct_device(),
                            );
                        }
                    }
                    let is_mic = state.audio_source == AudioSource::Microphone;
                    if ui.selectable_label(is_mic, "🎙 Microphone Input").clicked() {
                        state.audio_source = AudioSource::Microphone;
                        if is_active {
                            audio_worker.stop();
                            audio_worker.start(
                                state.style,
                                state.palette.clone(),
                                state.custom_tint,
                                state.sensitivity,
                                state.noise_gate,
                                state.fps,
                                Some(state.audio_source.clone()),
                                coordinator.direct_device(),
                            );
                        }
                    }
                });

                ui.add_space(8.0);

                // Sensitivity Slider
                ui.horizontal(|ui| {
                    ui.label("Sensitivity: ");
                    let changed = ui.add(
                        egui::Slider::new(&mut state.sensitivity, 0.2..=4.0)
                            .show_value(true)
                            .step_by(0.1)
                            .text("gain"),
                    );
                    if changed.changed() && is_active {
                        audio_worker.update_config(
                            state.style,
                            state.palette.clone(),
                            state.custom_tint,
                            state.sensitivity,
                            state.noise_gate,
                            state.fps,
                        );
                    }
                });

                // Noise Gate Slider
                ui.horizontal(|ui| {
                    ui.label("Noise Gate: ");
                    let changed = ui.add(
                        egui::Slider::new(&mut state.noise_gate, 0.0..=0.30)
                            .show_value(true)
                            .step_by(0.01)
                            .text("threshold"),
                    );
                    if changed.changed() && is_active {
                        audio_worker.update_config(
                            state.style,
                            state.palette.clone(),
                            state.custom_tint,
                            state.sensitivity,
                            state.noise_gate,
                            state.fps,
                        );
                    }
                });

                // Target Frame Rate
                ui.horizontal(|ui| {
                    ui.label("Target FPS:  ");
                    for target in [30, 60, 90] {
                        if ui.selectable_label(state.fps == target, format!("{} Hz", target)).clicked() {
                            state.fps = target;
                            if is_active {
                                audio_worker.update_config(
                                    state.style,
                                    state.palette.clone(),
                                    state.custom_tint,
                                    state.sensitivity,
                                    state.noise_gate,
                                    state.fps,
                                );
                            }
                        }
                    }
                });
            });

            // Right Column: Color Themes & Custom Palette Selection
            columns[1].vertical(|ui| {
                ui.label(
                    egui::RichText::new("Visualizer Color Theme & Palette")
                        .strong()
                        .color(ThemeColors::TEXT_SECONDARY),
                );
                ui.add_space(6.0);

                let palettes = [
                    ("Classic Stock", ColorPalette::StockClassic),
                    ("Cyberpunk Neon", ColorPalette::CyberpunkNeon),
                    ("Thermal / Fire", ColorPalette::ThermalFire),
                    ("Oceanic / Ice", ColorPalette::OceanicIce),
                ];

                for (p_name, p_val) in palettes {
                    let is_sel = state.palette == p_val;
                    if ui.selectable_label(is_sel, p_name).clicked() {
                        state.palette = p_val;
                        if is_active {
                            audio_worker.update_config(
                                state.style,
                                state.palette.clone(),
                                state.custom_tint,
                                state.sensitivity,
                                state.noise_gate,
                                state.fps,
                            );
                            let source_str = match &state.audio_source {
                                AudioSource::Microphone => "mic",
                                AudioSource::SystemLoopback => "system",
                                AudioSource::Custom(s) => s.as_str(),
                            };
                            let _ = coordinator.start_music(
                                state.style.name(),
                                state.custom_tint,
                                Some(state.palette.name()),
                                state.sensitivity,
                                Some(state.noise_gate),
                                Some(source_str),
                                state.fps,
                            );
                        }
                    }
                }

                // Custom Dual-Color Gradient Option
                let is_custom_grad = matches!(state.palette, ColorPalette::CustomGradient { .. });
                if ui.selectable_label(is_custom_grad, "🎨 Custom Dual-Color Gradient").clicked() {
                    let base = state.custom_tint.unwrap_or(Color::new(0, 50, 180));
                    let peak = state.custom_peak.unwrap_or(Color::new(0, 242, 254));
                    state.palette = ColorPalette::CustomGradient { base, peak };
                    if is_active {
                        audio_worker.update_config(
                            state.style,
                            state.palette.clone(),
                            state.custom_tint,
                            state.sensitivity,
                            state.noise_gate,
                            state.fps,
                        );
                    }
                }

                if let ColorPalette::CustomGradient { mut base, mut peak } = state.palette {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Base Color: ");
                        for (_, col) in crate::views::effects_studio::QUICK_SWATCHES.iter().take(6) {
                            let c32 = Color32::from_rgb(col.r, col.g, col.b);
                            let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
                            ui.painter().rect_filled(rect, Rounding::same(4.0), c32);
                            if base == *col {
                                ui.painter().rect_stroke(rect, Rounding::same(4.0), Stroke::new(2.0_f32, Color32::WHITE));
                            }
                            if resp.clicked() {
                                base = *col;
                                state.custom_tint = Some(base);
                                state.palette = ColorPalette::CustomGradient { base, peak };
                                if is_active {
                                    audio_worker.update_config(state.style, state.palette.clone(), state.custom_tint, state.sensitivity, state.noise_gate, state.fps);
                                }
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Peak Color: ");
                        for (_, col) in crate::views::effects_studio::QUICK_SWATCHES.iter().skip(4).take(6) {
                            let c32 = Color32::from_rgb(col.r, col.g, col.b);
                            let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::click());
                            ui.painter().rect_filled(rect, Rounding::same(4.0), c32);
                            if peak == *col {
                                ui.painter().rect_stroke(rect, Rounding::same(4.0), Stroke::new(2.0_f32, Color32::WHITE));
                            }
                            if resp.clicked() {
                                peak = *col;
                                state.custom_peak = Some(peak);
                                state.palette = ColorPalette::CustomGradient { base, peak };
                                if is_active {
                                    audio_worker.update_config(state.style, state.palette.clone(), state.custom_tint, state.sensitivity, state.noise_gate, state.fps);
                                }
                            }
                        }
                    });
                }
            });
        });
    });
}
