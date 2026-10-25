mod client;

use ajazz_audio::{AudioCapture, FFT_SIZE, MusicRenderer, SpectrumAnalyzer, VisualizerStyle};
use ajazz_hid::{enumerate_devices, AjazzDevice, HardwareConnection, PowerState};
use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{Direction, EffectMode, EffectSettings};
use ajazz_rgb::layout::{find_key_by_name, TOTAL_LEDS};
use clap::{Parser, Subcommand};
use client::{try_daemon_request, DaemonRequest, DaemonResponse};
use colored::*;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    name = "ak820ctl",
    author = "Antigravity Systems Team",
    version = "1.1.0",
    about = "Native Linux CLI control utility for Ajazz AK820 Max mechanical keyboards (Tri-Mode: Wired, 2.4G, Bluetooth)"
)]
struct Cli {
    #[arg(
        short,
        long,
        global = true,
        help = "Preferred connection mode: wired, 2.4g, bluetooth, auto"
    )]
    mode: Option<String>,

    #[arg(
        short,
        long,
        global = true,
        help = "Direct hidraw devnode path (e.g. /dev/hidraw1)"
    )]
    device: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Query and display keyboard status (battery, charging, connection)")]
    Status,

    #[command(about = "List all detected Ajazz keyboards, dongles, and interfaces")]
    Devices,

    #[command(about = "Switch or select active connection mode (wired, 2.4g, bluetooth, auto)")]
    Mode {
        #[arg(help = "Connection mode: wired, 2.4g, bluetooth, auto")]
        mode: String,
        #[arg(long, help = "Optional specific devnode (e.g. /dev/hidraw1)")]
        device: Option<String>,
    },

    #[command(about = "Set hardware lighting effect (e.g. rainbow, breathing, wave, static)")]
    Effect {
        #[arg(help = "Effect mode name (static, breathing, wave, spectrum, rain, reactive, etc.)")]
        mode: String,
        #[arg(short, long, help = "Brightness level (1..=5, default 5)")]
        brightness: Option<u8>,
        #[arg(short, long, help = "Speed level (1..=5, default 4)")]
        speed: Option<u8>,
        #[arg(long, help = "Direction (left or right)")]
        direction: Option<String>,
        #[arg(short, long, help = "Base color (hex or name: red, green, blue, rainbow, etc.)")]
        color: Option<String>,
    },

    #[command(about = "Set keyboard static color")]
    Static {
        #[arg(help = "Color (hex code like #00ffcc or name like red, cyan, purple)")]
        color: String,
    },

    #[command(about = "Start audio-reactive music RGB visualization")]
    Music {
        #[arg(short, long, default_value = "spectrum", help = "Style: spectrum, pulse, wave, soft-dance, passion")]
        style: String,
        #[arg(short, long, help = "Optional custom color tint (hex or name)")]
        color: Option<String>,
        #[arg(short, long, default_value_t = 30, help = "Target FPS (10..=60)")]
        fps: u32,
        #[arg(long, default_value_t = 1.0, help = "Microphone/audio sensitivity factor (0.1..=5.0)")]
        sensitivity: f32,
        #[arg(long, help = "Send to background daemon instead of running in foreground")]
        daemon: bool,
    },

    #[command(about = "Set color for an individual key or group (e.g. wasd, space, esc)")]
    Key {
        #[arg(help = "Key name (e.g. esc, space, a, f1, wasd)")]
        key: String,
        #[arg(help = "Color in hex (#FF0000) or name (red)")]
        color: String,
    },

    #[command(about = "Configure keyboard auto-sleep timeout")]
    Sleep {
        #[arg(help = "Sleep timeout in minutes (5, 15, 30, or 0 for never)")]
        minutes: u8,
    },

    #[command(about = "Interact with the background daemon (status, stop)")]
    Daemon {
        #[arg(help = "Action: ping, status, stop-music, shutdown")]
        action: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let pref_mode = cli.mode.as_deref().and_then(HardwareConnection::from_str_case);
    let dev_override = cli.device.as_deref();

    // If explicit mode or device was provided, notify daemon if available
    if cli.mode.is_some() || cli.device.is_some() {
        let mode_str = cli.mode.clone().unwrap_or_else(|| "auto".to_string());
        let _ = try_daemon_request(DaemonRequest::SetMode {
            mode: mode_str,
            device: cli.device.clone(),
        })
        .await;
    }

    match cli.command {
        Commands::Status => {
            // First try daemon
            if let Ok(DaemonResponse::Status(st)) = try_daemon_request(DaemonRequest::GetStatus).await {
                print_status(&st, "Daemon (via IPC)");
                return Ok(());
            }

            // Direct device query with preference
            let dev = match AjazzDevice::open_with_preference(pref_mode, dev_override) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{} {}", "Error opening device:".red().bold(), e);
                    std::process::exit(1);
                }
            };

            let info = dev.info().clone();
            match dev.status() {
                Ok(st) => print_status(&st, &format!("Direct ({}, {:?})", info.device_name, info.connection_mode)),
                Err(e) => eprintln!("{} {}", "Failed to query device status:".red().bold(), e),
            }
        }

        Commands::Devices => {
            let devs = enumerate_devices();
            println!("{}", "=== Detected Ajazz AK820 Devices ===".cyan().bold());
            if devs.is_empty() {
                println!("{}", "No Ajazz devices found. Check USB connection or udev rules.".yellow());
            } else {
                for (i, d) in devs.iter().enumerate() {
                    let mode_badge = match d.connection_mode {
                        HardwareConnection::Wired => "[USB Wired]".green().bold(),
                        HardwareConnection::Wireless24G => "[2.4GHz Wireless]".magenta().bold(),
                        HardwareConnection::Bluetooth => "[Bluetooth]".blue().bold(),
                        HardwareConnection::Unknown => "[Unknown]".yellow().bold(),
                    };

                    println!(
                        "[{}] {} {}\n    Devnode: {}\n    Hardware ID: {:04X}:{:04X} (Bus {})\n    Interface: {:?}\n    Protocol: {:?}\n    Usage Pages: 0x{:04X?}",
                        i + 1,
                        d.device_name.bold(),
                        mode_badge,
                        d.devnode.cyan(),
                        d.vendor_id,
                        d.product_id,
                        d.bus_type,
                        d.interface,
                        d.protocol,
                        d.usage_pages
                    );
                }
            }
        }

        Commands::Mode { mode, device } => {
            let req = DaemonRequest::SetMode {
                mode: mode.clone(),
                device: device.clone(),
            };
            match try_daemon_request(req).await {
                Ok(DaemonResponse::Ok { message }) => {
                    println!("{} {}", "Success:".green().bold(), message);
                }
                Ok(DaemonResponse::Error { message }) => {
                    eprintln!("{} {}", "Daemon Error:".red().bold(), message);
                }
                _ => {
                    let parsed = HardwareConnection::from_str_case(&mode);
                    let dev = AjazzDevice::open_with_preference(parsed, device.as_deref())?;
                    println!(
                        "{} Connected directly to {} on {} (mode: {:?})",
                        "Success:".green().bold(),
                        dev.info().device_name,
                        dev.info().devnode,
                        dev.connection_mode()
                    );
                }
            }
        }

        Commands::Effect {
            mode,
            brightness,
            speed,
            direction,
            color,
        } => {
            // Try via daemon first if no explicit devnode was requested
            if dev_override.is_none() {
                if let Ok(resp) = try_daemon_request(DaemonRequest::SetEffect {
                    mode: mode.clone(),
                    brightness,
                    speed,
                    direction: direction.clone(),
                    color: color.clone(),
                })
                .await
                {
                    match resp {
                        DaemonResponse::Ok { message } => {
                            println!("{} {}", "Success:".green().bold(), message);
                            return Ok(());
                        }
                        DaemonResponse::Error { message } => {
                            eprintln!("{} {}", "Daemon error:".red().bold(), message);
                        }
                        _ => {}
                    }
                }
            }

            // Direct fallback
            let effect_mode = EffectMode::from_name(&mode)
                .ok_or_else(|| anyhow::anyhow!("Unknown effect mode '{}'", mode))?;

            let dir = match direction.as_deref() {
                Some("left") => Direction::Left,
                _ => Direction::Right,
            };

            let (palette, custom_c) = if let Some(c_str) = color {
                if let Ok(c) = Color::from_str(&c_str) {
                    (PaletteColor::Rainbow, Some(c))
                } else {
                    (PaletteColor::Rainbow, None)
                }
            } else {
                (PaletteColor::Rainbow, None)
            };

            let settings = EffectSettings {
                mode: effect_mode,
                brightness: brightness.unwrap_or(5),
                speed: speed.unwrap_or(4),
                direction: dir,
                palette_color: palette,
                custom_color: custom_c,
            };

            let dev = AjazzDevice::open_with_preference(pref_mode, dev_override)?;
            dev.set_lighting(&settings)?;
            println!(
                "{} Applied effect '{}' on {} ({:?})",
                "Success:".green().bold(),
                effect_mode.name(),
                dev.info().devnode,
                dev.connection_mode()
            );
        }

        Commands::Static { color } => {
            if dev_override.is_none() {
                if let Ok(resp) = try_daemon_request(DaemonRequest::SetStatic { color: color.clone() }).await {
                    match resp {
                        DaemonResponse::Ok { message } => {
                            println!("{} {}", "Success:".green().bold(), message);
                            return Ok(());
                        }
                        DaemonResponse::Error { message } => {
                            eprintln!("{} {}", "Daemon error:".red().bold(), message);
                        }
                        _ => {}
                    }
                }
            }

            let c = Color::from_str(&color)?;
            let settings = EffectSettings {
                mode: EffectMode::Static,
                brightness: 5,
                speed: 4,
                direction: Direction::Right,
                palette_color: PaletteColor::Rainbow,
                custom_color: Some(c),
            };

            let dev = AjazzDevice::open_with_preference(pref_mode, dev_override)?;
            dev.set_lighting(&settings)?;
            println!(
                "{} Set static color to {} on {} ({:?})",
                "Success:".green().bold(),
                c,
                dev.info().devnode,
                dev.connection_mode()
            );
        }

        Commands::Music {
            style,
            color,
            fps,
            sensitivity,
            daemon,
        } => {
            if daemon {
                let req = DaemonRequest::StartMusic {
                    style: style.clone(),
                    color: color.clone(),
                    sensitivity: Some(sensitivity),
                    fps: Some(fps),
                };
                match try_daemon_request(req).await {
                    Ok(DaemonResponse::Ok { message }) => println!("{} {}", "Success:".green().bold(), message),
                    Ok(DaemonResponse::Error { message }) => eprintln!("{} {}", "Error:".red().bold(), message),
                    Err(e) => eprintln!("{} Could not contact daemon: {}", "Error:".red().bold(), e),
                    _ => {}
                }
                return Ok(());
            }

            // Foreground visualization
            println!(
                "{} Starting interactive foreground music visualization ({})",
                "Music Studio:".magenta().bold(),
                style.cyan()
            );

            let vis_style = VisualizerStyle::from_name(&style)
                .ok_or_else(|| anyhow::anyhow!("Unknown visualizer style '{}'", style))?;

            let custom_tint = color.and_then(|c| Color::from_str(&c).ok());

            let dev = AjazzDevice::open_with_preference(pref_mode, dev_override)?;
            println!("Target hardware: {} ({:?})", dev.info().devnode.cyan(), dev.connection_mode());
            dev.enter_live_music_mode()?;

            let capture = AudioCapture::new(None)?;
            println!("Default audio source initialized @ {} Hz", capture.sample_rate());

            let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), sensitivity);
            let mut renderer = MusicRenderer::new(vis_style, custom_tint);

            let target_fps = fps.clamp(10, 60);
            let frame_dur = Duration::from_micros((1_000_000 / target_fps as u64).max(16_666));

            println!("Streaming to keyboard at ~{} FPS. Press Ctrl+C to stop.", target_fps);

            loop {
                let start = Instant::now();
                let pcm = capture.read_latest(FFT_SIZE);
                if !pcm.is_empty() {
                    analyzer.update(&pcm);
                }

                let frame = renderer.render(&analyzer, 0.033);
                let _ = dev.send_live_frame(&frame);

                let elapsed = start.elapsed();
                if elapsed < frame_dur {
                    std::thread::sleep(frame_dur - elapsed);
                }
            }
        }

        Commands::Key { key, color } => {
            let c = Color::from_str(&color)?;
            let key_info = find_key_by_name(&key)
                .ok_or_else(|| anyhow::anyhow!("Unknown key identifier '{}'", key))?;

            if dev_override.is_none() {
                let req = DaemonRequest::SetCustomKey {
                    key: key.clone(),
                    color: color.clone(),
                };
                if let Ok(DaemonResponse::Ok { message }) = try_daemon_request(req).await {
                    println!("{} {}", "Success:".green().bold(), message);
                    return Ok(());
                }
            }

            let dev = AjazzDevice::open_with_preference(pref_mode, dev_override)?;
            let mut matrix = [Color::BLACK; TOTAL_LEDS];
            matrix[key_info.led_idx] = c;
            dev.set_custom_matrix(&matrix, 5)?;
            println!("{} Set key '{}' to {}", "Success:".green().bold(), key_info.label, c);
        }

        Commands::Sleep { minutes } => {
            if dev_override.is_none() {
                let req = DaemonRequest::SetSleep { minutes };
                if let Ok(DaemonResponse::Ok { message }) = try_daemon_request(req).await {
                    println!("{} {}", "Success:".green().bold(), message);
                    return Ok(());
                }
            }

            let dev = AjazzDevice::open_with_preference(pref_mode, dev_override)?;
            dev.set_sleep_timeout(minutes)?;
            println!("{} Set sleep timeout to {} min", "Success:".green().bold(), minutes);
        }

        Commands::Daemon { action } => match action.as_str() {
            "ping" => match try_daemon_request(DaemonRequest::Ping).await {
                Ok(DaemonResponse::Pong) => println!("{}", "Daemon is running (PONG)".green().bold()),
                _ => println!("{}", "Daemon is unreachable".red().bold()),
            },
            "status" => match try_daemon_request(DaemonRequest::GetStatus).await {
                Ok(DaemonResponse::Status(st)) => print_status(&st, "Daemon"),
                Ok(DaemonResponse::Error { message }) => eprintln!("{} {}", "Daemon error:".red().bold(), message),
                _ => println!("{}", "Daemon is unreachable".red().bold()),
            },
            "stop-music" => match try_daemon_request(DaemonRequest::StopMusic).await {
                Ok(DaemonResponse::Ok { message }) => println!("{} {}", "Success:".green().bold(), message),
                _ => eprintln!("{}", "Failed to stop music or daemon unreachable".red()),
            },
            "shutdown" => match try_daemon_request(DaemonRequest::Shutdown).await {
                Ok(_) => println!("{}", "Daemon shutdown requested".yellow()),
                _ => println!("{}", "Daemon already offline".yellow()),
            },
            other => eprintln!("Unknown action '{}'. Valid actions: ping, status, stop-music, shutdown", other),
        },
    }

    Ok(())
}

fn print_status(status: &ajazz_hid::DeviceStatus, source: &str) {
    println!("{}", "=== Ajazz AK820 Max Status ===".cyan().bold());
    println!("  Source:          {}", source);
    if let Some(bat) = status.battery_percentage {
        let bat_str = format!("{}%", bat);
        let colored_bat = if bat > 50 {
            bat_str.green()
        } else if bat > 20 {
            bat_str.yellow()
        } else {
            bat_str.red()
        };
        println!("  Battery:         {}", colored_bat);
    } else {
        println!("  Battery:         {}", "Unknown / Wired".dimmed());
    }

    let pwr_str = match status.power_state {
        PowerState::Charging => "Charging".yellow().bold(),
        PowerState::OnBattery => "On Battery".blue(),
        PowerState::FullyCharged => "Fully Charged / Cable Plugged".green().bold(),
        PowerState::Unknown => "Unknown".dimmed(),
    };
    println!("  Power State:     {}", pwr_str);
}
