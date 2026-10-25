mod client;

use ajazz_audio::{AudioCapture, FFT_SIZE, MusicRenderer, SpectrumAnalyzer, VisualizerStyle};
use ajazz_hid::{enumerate_devices, AjazzDevice, PowerState};
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
    version = "0.1.0",
    about = "Native Linux CLI control utility for Ajazz AK820 Max mechanical keyboards"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Query and display keyboard status (battery, charging, connection)")]
    Status,

    #[command(about = "List all detected Ajazz keyboards and USB interfaces")]
    Devices,

    #[command(about = "Set hardware lighting effect (e.g. rainbow, breathing, wave, static)")]
    Effect {
        #[arg(help = "Effect mode name (static, breathing, wave, spectrum, rain, reactive, etc.)")]
        mode: String,
        #[arg(short, long, help = "Brightness level (1..=5, default 5)")]
        brightness: Option<u8>,
        #[arg(short, long, help = "Speed level (1..=5, default 4)")]
        speed: Option<u8>,
        #[arg(short, long, help = "Direction (left or right)")]
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
        #[arg(short, long, default_value_t = 1.0, help = "Microphone/audio sensitivity factor (0.1..=5.0)")]
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

    match cli.command {
        Commands::Status => {
            // First try daemon
            if let Ok(DaemonResponse::Status(st)) = try_daemon_request(DaemonRequest::GetStatus).await {
                print_status(&st, "Daemon (via IPC)");
                return Ok(());
            }

            // Direct device query
            let dev = match AjazzDevice::open_auto() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{} {}", "Error opening device:".red().bold(), e);
                    std::process::exit(1);
                }
            };

            let info = dev.info().clone();
            match dev.status() {
                Ok(st) => print_status(&st, &format!("Direct ({})", info.device_name)),
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
                    let mode_str = if d.is_wired { "Wired USB".green() } else { "2.4GHz Wireless".magenta() };
                    println!(
                        "[{}] {} ({:04X}:{:04X})\n    Devnode: {}\n    Mode: {}\n    Protocol: {:?}\n    Usage Page: 0x{:04X?}, Usage: 0x{:04X?}",
                        i + 1,
                        d.device_name.bold(),
                        d.vendor_id,
                        d.product_id,
                        d.devnode.cyan(),
                        mode_str,
                        d.protocol,
                        d.usage_page.unwrap_or(0),
                        d.usage.unwrap_or(0)
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
            // Try via daemon first
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
                    DaemonResponse::Ok { message } => println!("{} {}", "Success:".green().bold(), message),
                    DaemonResponse::Error { message } => eprintln!("{} {}", "Daemon error:".red().bold(), message),
                    _ => {}
                }
                return Ok(());
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

            let dev = AjazzDevice::open_auto()?;
            dev.set_lighting(&settings)?;
            println!("{} Applied effect '{}'", "Success:".green().bold(), effect_mode.name());
        }

        Commands::Static { color } => {
            if let Ok(resp) = try_daemon_request(DaemonRequest::SetStatic { color: color.clone() }).await {
                match resp {
                    DaemonResponse::Ok { message } => println!("{} {}", "Success:".green().bold(), message),
                    DaemonResponse::Error { message } => eprintln!("{} {}", "Daemon error:".red().bold(), message),
                    _ => {}
                }
                return Ok(());
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

            let dev = AjazzDevice::open_auto()?;
            dev.set_lighting(&settings)?;
            println!("{} Set keyboard static color to {}", "Success:".green().bold(), c);
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
                    Err(e) => eprintln!("{} Could not reach daemon: {}. Run without --daemon for foreground mode.", "Warning:".yellow().bold(), e),
                    _ => {}
                }
                return Ok(());
            }

            // Foreground live music visualization
            run_foreground_music(style, color, fps, sensitivity).await?;
        }

        Commands::Key { key, color } => {
            let c = Color::from_str(&color)?;
            let keys_to_set = if key.to_lowercase() == "wasd" {
                vec!["w", "a", "s", "d"]
            } else if key.to_lowercase() == "arrows" {
                vec!["up", "down", "left", "right"]
            } else {
                vec![key.as_str()]
            };

            let mut matrix = [Color::BLACK; TOTAL_LEDS];
            for k_name in keys_to_set {
                if let Some(k) = find_key_by_name(k_name) {
                    matrix[k.led_idx] = c;
                    println!("Set key {} -> {}", k.label.bold(), c);
                } else {
                    eprintln!("{} Key '{}' not recognized", "Warning:".yellow().bold(), k_name);
                }
            }

            let dev = AjazzDevice::open_auto()?;
            dev.set_custom_matrix(&matrix, 5)?;
            println!("{}", "Uploaded custom matrix to keyboard.".green().bold());
        }

        Commands::Sleep { minutes } => {
            if let Ok(resp) = try_daemon_request(DaemonRequest::SetSleep { minutes }).await {
                if let DaemonResponse::Ok { message } = resp {
                    println!("{} {}", "Success:".green().bold(), message);
                    return Ok(());
                }
            }

            let dev = AjazzDevice::open_auto()?;
            dev.set_sleep_timeout(minutes)?;
            println!("{} Configured sleep timeout to {} min", "Success:".green().bold(), minutes);
        }

        Commands::Daemon { action } => match action.to_lowercase().as_str() {
            "ping" => match try_daemon_request(DaemonRequest::Ping).await {
                Ok(DaemonResponse::Pong) => println!("{}", "Daemon is running (PONG)".green().bold()),
                Ok(other) => println!("Daemon response: {:?}", other),
                Err(e) => eprintln!("Daemon error: {}", e),
            },
            "stop-music" => match try_daemon_request(DaemonRequest::StopMusic).await {
                Ok(DaemonResponse::Ok { message }) => println!("{}", message.green().bold()),
                Err(e) => eprintln!("Daemon error: {}", e),
                _ => {}
            },
            "shutdown" => match try_daemon_request(DaemonRequest::Shutdown).await {
                Ok(DaemonResponse::Ok { message }) => println!("{}", message.green().bold()),
                Err(e) => eprintln!("Daemon error: {}", e),
                _ => {}
            },
            other => eprintln!("Unknown daemon action '{}'. Try: ping, stop-music, shutdown", other),
        },
    }

    Ok(())
}

fn print_status(st: &ajazz_hid::DeviceStatus, source: &str) {
    println!("{}", "=== Ajazz AK820 Max Status ===".cyan().bold());
    println!("  Source:          {}", source.yellow());

    if let Some(bat) = st.battery_percentage {
        let bat_str = format!("{}%", bat);
        let colored_bat = if bat > 50 {
            bat_str.green().bold()
        } else if bat > 20 {
            bat_str.yellow().bold()
        } else {
            bat_str.red().bold()
        };
        println!("  Battery:         {}", colored_bat);
    } else {
        println!("  Battery:         {}", "N/A (Wired mode)".dimmed());
    }

    let charging_str = match st.power_state {
        PowerState::Charging => "Charging (Plugged in)".green().bold(),
        PowerState::FullyCharged => "Fully Charged".green().bold(),
        PowerState::OnBattery => "On Battery".yellow(),
        PowerState::Unknown => "Connected".white(),
    };
    println!("  Power State:     {}", charging_str);
}

async fn run_foreground_music(
    style_str: String,
    color_str: Option<String>,
    fps: u32,
    sensitivity: f32,
) -> anyhow::Result<()> {
    let style = VisualizerStyle::from_name(&style_str)
        .ok_or_else(|| anyhow::anyhow!("Unknown visualizer style '{}'", style_str))?;

    let custom_col = color_str.and_then(|c| Color::from_str(&c).ok());
    let dev = AjazzDevice::open_auto()?;
    println!("{}", "Connecting to audio stream...".cyan());

    let capture = AudioCapture::new(None)?;
    dev.enter_live_music_mode()?;

    println!(
        "{} Live music visualization running [Style: {}, FPS: {}]",
        "Active:".green().bold(),
        style.name().bold(),
        fps
    );
    println!("{}", "Press Ctrl+C to stop.".dimmed());

    let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), sensitivity);
    let mut renderer = MusicRenderer::new(style, custom_col);

    let period = Duration::from_micros(1_000_000 / fps.clamp(10, 60) as u64);
    let start_time = Instant::now();

    loop {
        let loop_start = Instant::now();
        let pcm = capture.read_latest(FFT_SIZE * 2);

        analyzer.update(&pcm);
        let t = start_time.elapsed().as_secs_f32();
        let frame = renderer.render(&analyzer, t);

        if let Err(e) = dev.send_live_frame(&frame) {
            eprintln!("Warning sending frame: {}", e);
        }

        // Render console ASCII meter
        print!("\r[Audio Energy] [");
        let bars = (analyzer.energy * 30.0).round() as usize;
        for i in 0..30 {
            if i < bars {
                print!("{}", "=".green());
            } else {
                print!(" ");
            }
        }
        print!("] Bass: {:.2}   ", analyzer.bass);
        use std::io::Write;
        std::io::stdout().flush().ok();

        let elapsed = loop_start.elapsed();
        if elapsed < period {
            tokio::time::sleep(period - elapsed).await;
        }
    }
}
