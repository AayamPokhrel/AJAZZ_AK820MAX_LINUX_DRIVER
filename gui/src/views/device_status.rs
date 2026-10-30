use crate::ipc::{ConnectionMode, HardwareCoordinator};
use crate::theme::ThemeColors;
use ajazz_hid::HardwareConnection;
use egui::{pos2, vec2, Rect, Rounding, Stroke, Ui};

pub fn render_device_header(ui: &mut Ui, coordinator: &mut HardwareCoordinator) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            // App Branding & Title
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("AJAZZ AK820 MAX")
                        .heading()
                        .color(ThemeColors::TEXT_PRIMARY)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("RGB STUDIO v1.2.0")
                        .small()
                        .color(ThemeColors::ACCENT_CYAN),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Re-scan / Refresh button
                let rescan_btn = egui::Button::new("⟳ Re-scan")
                    .rounding(Rounding::same(6.0))
                    .min_size(vec2(72.0, 26.0));
                if ui.add(rescan_btn).clicked() {
                    coordinator.refresh();
                }

                ui.separator();

                // Battery Indicator
                if let Some(status) = &coordinator.status {
                    let pct = status.battery_percentage.unwrap_or(100).clamp(0, 100);
                    let (bat_color, bat_symbol) = if status.is_charging {
                        (ThemeColors::ACCENT_CYAN, "⚡ Charging")
                    } else if pct >= 50 {
                        (ThemeColors::STATUS_SUCCESS, "🔋")
                    } else if pct >= 20 {
                        (ThemeColors::STATUS_WARNING, "🪫")
                    } else {
                        (ThemeColors::STATUS_ERROR, "⚠️ Low")
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{} {}%", bat_symbol, pct))
                                .strong()
                                .color(bat_color),
                        );

                        // Mini battery bar gauge
                        let (gauge_rect, _) = ui.allocate_exact_size(vec2(36.0, 14.0), egui::Sense::hover());
                        ui.painter().rect(
                            gauge_rect,
                            Rounding::same(3.0),
                            ThemeColors::BG_MAIN,
                            Stroke::new(1.0_f32, ThemeColors::BORDER_MUTED),
                        );
                        let fill_w = (gauge_rect.width() - 4.0) * (pct as f32 / 100.0);
                        let fill_rect = Rect::from_min_size(
                            pos2(gauge_rect.min.x + 2.0, gauge_rect.min.y + 2.0),
                            vec2(fill_w, gauge_rect.height() - 4.0),
                        );
                        ui.painter().rect_filled(fill_rect, Rounding::same(2.0), bat_color);
                    });

                    ui.separator();
                }

                // Connection Mode Badge
                let (conn_text, conn_color) = match coordinator.mode {
                    ConnectionMode::ConnectedDaemon => ("● ak820d IPC", ThemeColors::STATUS_SUCCESS),
                    ConnectionMode::ConnectedDirectHid => ("● Direct HID", ThemeColors::ACCENT_CYAN),
                    ConnectionMode::Disconnected => ("○ Disconnected", ThemeColors::STATUS_ERROR),
                };

                ui.label(
                    egui::RichText::new(conn_text)
                        .color(conn_color)
                        .strong(),
                );

                // Hardware Transport details (Wired vs 2.4G vs Bluetooth)
                if let Some(info) = &coordinator.device_info {
                    let (transport_badge, badge_color) = match info.connection_mode {
                        HardwareConnection::Wired => ("● USB Wired", ThemeColors::ACCENT_CYAN),
                        HardwareConnection::Wireless24G => ("● 2.4GHz Dongle", ThemeColors::ACCENT_PURPLE),
                        HardwareConnection::Bluetooth => ("● Bluetooth", ThemeColors::STATUS_SUCCESS),
                        HardwareConnection::Unknown => ("● Keyboard", ThemeColors::TEXT_SECONDARY),
                    };
                    ui.label(
                        egui::RichText::new(transport_badge)
                            .small()
                            .strong()
                            .color(badge_color),
                    );
                }
            });
        });
    });
}

pub fn render_hardware_settings_panel(
    ui: &mut Ui,
    coordinator: &mut HardwareCoordinator,
    sleep_minutes: &mut u8,
    status_msg: &mut Option<(String, bool)>,
) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("Hardware & Connection Configuration")
                    .strong()
                    .color(ThemeColors::TEXT_PRIMARY),
            );

            ui.add_space(8.0);

            // Active Connection Switcher
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Active Mode:").strong());

                let cur_pref = coordinator.preferred_connection;
                if ui.selectable_label(cur_pref.is_none(), "⚡ Auto-Detect").clicked() {
                    coordinator.set_hardware_mode(None);
                    *status_msg = Some(("Connection set to Auto-Detect".to_string(), true));
                }
                if ui.selectable_label(cur_pref == Some(HardwareConnection::Wired), "🔌 USB Wired").clicked() {
                    coordinator.set_hardware_mode(Some(HardwareConnection::Wired));
                    *status_msg = Some(("Switched to USB Wired mode".to_string(), true));
                }
                if ui.selectable_label(cur_pref == Some(HardwareConnection::Wireless24G), "📡 2.4GHz Wireless").clicked() {
                    coordinator.set_hardware_mode(Some(HardwareConnection::Wireless24G));
                    *status_msg = Some(("Switched to 2.4GHz Wireless mode".to_string(), true));
                }
                if ui.selectable_label(cur_pref == Some(HardwareConnection::Bluetooth), "ᛒ Bluetooth").clicked() {
                    coordinator.set_hardware_mode(Some(HardwareConnection::Bluetooth));
                    *status_msg = Some(("Switched to Bluetooth mode".to_string(), true));
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            if let Some(info) = &coordinator.device_info {
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        ui.label(format!("Device Node: {}", info.devnode));
                        ui.label(format!("Connection: {:?}", info.connection_mode));
                        ui.label(format!("Hardware Protocol: {:?}", info.protocol));
                        ui.label(format!(
                            "Hardware ID: {:04X}:{:04X} (Bus {})",
                            info.vendor_id, info.product_id, info.bus_type
                        ));
                    });

                    cols[1].vertical(|ui| {
                        if let Some(status) = &coordinator.status {
                            ui.label(format!("Battery Percentage: {}%", status.battery_percentage.unwrap_or(100)));
                            ui.label(format!(
                                "Power State: {:?} ({})",
                                status.power_state,
                                if status.is_charging { "Charging / Powered" } else { "Discharging" }
                            ));
                        }
                        ui.label(format!(
                            "Active Sleep Timer: {} minutes",
                            if *sleep_minutes == 0 {
                                "Never (Always On)".to_string()
                            } else {
                                format!("{}m", sleep_minutes)
                            }
                        ));
                    });
                });
            } else {
                ui.label(
                    egui::RichText::new("No AK820 Max keyboard detected.")
                        .color(ThemeColors::STATUS_WARNING),
                );
                ui.label(
                    "Please ensure the USB-C cable or 2.4GHz USB dongle is connected, and your user has access to /dev/hidraw*.",
                );
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            // Auto-sleep timer selection
            ui.horizontal(|ui| {
                ui.label("Wireless Auto-Sleep Timer:");
                for (label, mins) in [("5 min", 5), ("15 min", 15), ("30 min", 30), ("Never (Always On)", 0)] {
                    if ui.selectable_label(*sleep_minutes == mins, label).clicked() {
                        *sleep_minutes = mins;
                        match coordinator.set_sleep_timeout(mins) {
                            Ok(_) => {
                                *status_msg = Some((format!("Sleep timeout set to {}", label), true));
                            }
                            Err(e) => {
                                *status_msg = Some((format!("Failed to set sleep: {}", e), false));
                            }
                        }
                    }
                }
            });
        });
    });
}
