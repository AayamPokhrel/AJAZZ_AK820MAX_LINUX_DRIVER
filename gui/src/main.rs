mod app;
mod audio_worker;
mod ipc;
mod theme;
mod views;

use app::{Ak820App, AppTab};
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "ak820-gui")]
#[command(author = "Ajazz Linux Community")]
#[command(version = "1.1.1")]
#[command(about = "Native Linux GUI for Ajazz AK820 Max Keyboard RGB & Music Visualizer")]
struct CliArgs {
    /// Initial tab to open (effects, music, canvas, hardware)
    #[arg(short, long)]
    tab: Option<String>,
}

fn load_window_icon() -> Option<egui::IconData> {
    // Embedded 256x256 PNG app icon
    let icon_bytes = include_bytes!("../../assets/icons/ajazz-ak820-max.png");
    match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            })
        }
        Err(e) => {
            eprintln!("Warning: Failed to parse embedded icon: {}", e);
            None
        }
    }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = CliArgs::parse();

    let initial_tab = match args.tab.as_deref() {
        Some("music") | Some("audio") => Some(AppTab::Music),
        Some("canvas") | Some("paint") | Some("custom") => Some(AppTab::CustomCanvas),
        Some("hardware") | Some("settings") | Some("battery") => Some(AppTab::Hardware),
        Some("effects") | Some("lights") => Some(AppTab::Effects),
        _ => None,
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Ajazz AK820 Max RGB Studio")
        .with_inner_size([880.0, 720.0])
        .with_min_inner_size([640.0, 480.0])
        .with_app_id("ak820-gui");

    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ak820-gui",
        native_options,
        Box::new(move |cc| Ok(Box::new(Ak820App::new(cc, initial_tab)))),
    )
}
