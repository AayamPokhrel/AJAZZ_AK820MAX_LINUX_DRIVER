mod ipc;

use ajazz_audio::{AudioCapture, FFT_SIZE, MusicRenderer, SpectrumAnalyzer, VisualizerStyle};
use ajazz_hid::{AjazzDevice, DeviceError};
use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{Direction, EffectMode, EffectSettings};
use ajazz_rgb::layout::{find_key_by_name, TOTAL_LEDS};
use clap::Parser;
use ipc::{default_socket_path, DaemonRequest, DaemonResponse};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "ak820d", about = "Ajazz AK820 Max native Linux background daemon")]
struct Args {
    #[arg(short, long, help = "Custom Unix domain socket path")]
    socket: Option<PathBuf>,

    #[arg(short, long, help = "Enable verbose debug logging")]
    verbose: bool,
}

struct DaemonState {
    device: Option<Arc<AjazzDevice>>,
    music_active: Arc<AtomicBool>,
    music_handle: Option<std::thread::JoinHandle<()>>,
    custom_matrix: [Color; TOTAL_LEDS],
    last_settings: EffectSettings,
}

impl DaemonState {
    fn new() -> Self {
        Self {
            device: None,
            music_active: Arc::new(AtomicBool::new(false)),
            music_handle: None,
            custom_matrix: [Color::BLACK; TOTAL_LEDS],
            last_settings: EffectSettings::default(),
        }
    }

    fn ensure_device(&mut self) -> Result<Arc<AjazzDevice>, DeviceError> {
        if self.device.is_none() {
            let dev = Arc::new(AjazzDevice::open_auto()?);
            self.device = Some(dev);
        }
        Ok(Arc::clone(self.device.as_ref().unwrap()))
    }

    fn stop_music(&mut self) {
        if self.music_active.swap(false, Ordering::SeqCst) {
            info!("Stopping background music visualization");
            if let Some(handle) = self.music_handle.take() {
                let _ = handle.join();
            }
            let last_settings = self.last_settings.clone();
            if let Ok(dev) = self.ensure_device() {
                let _ = dev.set_lighting(&last_settings);
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let env_filter = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(env_filter)),
        )
        .init();

    info!("Starting Ajazz AK820 Max userspace driver daemon");

    let socket_path = args.socket.unwrap_or_else(default_socket_path);
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(&socket_path)?;
    info!("Daemon listening on Unix socket: {}", socket_path.display());

    let state = Arc::new(Mutex::new(DaemonState::new()));

    // Try initial connection
    {
        let mut st = state.lock().await;
        match st.ensure_device() {
            Ok(dev) => info!("Connected to device: {}", dev.info().device_name),
            Err(e) => warn!("Initial device connection failed: {}. Will retry on demand.", e),
        }
    }

    let state_clone = Arc::clone(&state);

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal (Ctrl+C)");
        }
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let st_ref = Arc::clone(&state_clone);
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, st_ref).await {
                                error!("Error handling client request: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        } => {}
    }

    info!("Cleaning up daemon resources");
    let mut st = state.lock().await;
    st.stop_music();
    let _ = std::fs::remove_file(&socket_path);
    info!("Daemon terminated gracefully");

    Ok(())
}

async fn handle_client(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<DaemonState>>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let req: DaemonRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = DaemonResponse::Error {
                    message: format!("Malformed JSON request: {}", e),
                };
                let mut data = serde_json::to_vec(&resp)?;
                data.push(b'\n');
                writer.write_all(&data).await?;
                continue;
            }
        };

        let resp = process_request(req, &state).await;
        let mut out = serde_json::to_vec(&resp)?;
        out.push(b'\n');
        writer.write_all(&out).await?;
    }

    Ok(())
}

async fn process_request(req: DaemonRequest, state: &Arc<Mutex<DaemonState>>) -> DaemonResponse {
    let mut st = state.lock().await;

    match req {
        DaemonRequest::Ping => DaemonResponse::Pong,

        DaemonRequest::GetStatus => match st.ensure_device() {
            Ok(dev) => match dev.status() {
                Ok(status) => DaemonResponse::Status(status),
                Err(e) => DaemonResponse::Error {
                    message: format!("Failed to read status: {}", e),
                },
            },
            Err(e) => DaemonResponse::Error {
                message: format!("Device not connected: {}", e),
            },
        },

        DaemonRequest::SetEffect {
            mode,
            brightness,
            speed,
            direction,
            color,
        } => {
            st.stop_music();
            let effect_mode = match EffectMode::from_name(&mode) {
                Some(m) => m,
                None => {
                    return DaemonResponse::Error {
                        message: format!("Unknown effect mode: {}", mode),
                    };
                }
            };

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

            st.last_settings = settings.clone();

            match st.ensure_device() {
                Ok(dev) => match dev.set_lighting(&settings) {
                    Ok(_) => DaemonResponse::Ok {
                        message: format!("Set effect to {:?}", effect_mode),
                    },
                    Err(e) => DaemonResponse::Error {
                        message: format!("Failed to set effect: {}", e),
                    },
                },
                Err(e) => DaemonResponse::Error {
                    message: format!("Device not connected: {}", e),
                },
            }
        }

        DaemonRequest::SetStatic { color } => {
            st.stop_music();
            let c = match Color::from_str(&color) {
                Ok(c) => c,
                Err(e) => {
                    return DaemonResponse::Error {
                        message: format!("Invalid color '{}': {}", color, e),
                    };
                }
            };

            let settings = EffectSettings {
                mode: EffectMode::Static,
                brightness: 5,
                speed: 4,
                direction: Direction::Right,
                palette_color: PaletteColor::Rainbow,
                custom_color: Some(c),
            };
            st.last_settings = settings.clone();

            match st.ensure_device() {
                Ok(dev) => match dev.set_lighting(&settings) {
                    Ok(_) => DaemonResponse::Ok {
                        message: format!("Set static color to {}", c),
                    },
                    Err(e) => DaemonResponse::Error {
                        message: format!("Failed to set static color: {}", e),
                    },
                },
                Err(e) => DaemonResponse::Error {
                    message: format!("Device not connected: {}", e),
                },
            }
        }

        DaemonRequest::StartMusic {
            style,
            color,
            sensitivity,
            fps,
        } => {
            st.stop_music();

            let vis_style = match VisualizerStyle::from_name(&style) {
                Some(s) => s,
                None => {
                    return DaemonResponse::Error {
                        message: format!("Unknown visualizer style: {}", style),
                    };
                }
            };

            let custom_col = color.and_then(|c| Color::from_str(&c).ok());
            let target_fps = fps.unwrap_or(30).clamp(10, 60);
            let sens = sensitivity.unwrap_or(1.0);

            let dev = match st.ensure_device() {
                Ok(d) => d,
                Err(e) => {
                    return DaemonResponse::Error {
                        message: format!("Cannot start music sync; device unavailable: {}", e),
                    };
                }
            };

            if let Err(e) = dev.enter_live_music_mode() {
                return DaemonResponse::Error {
                    message: format!("Failed to enter live music mode: {}", e),
                };
            }

            let active_flag = Arc::clone(&st.music_active);
            active_flag.store(true, Ordering::SeqCst);

            let dev_clone = Arc::clone(&dev);

            let handle = std::thread::spawn(move || {
                let capture = match AudioCapture::new(None) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to initialize audio capture inside thread: {}", e);
                        active_flag.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), sens);
                let mut renderer = MusicRenderer::new(vis_style, custom_col);

                let period = Duration::from_micros(1_000_000 / target_fps as u64);
                let start_time = Instant::now();

                info!(
                    "Music reactive stream active at {} FPS (style: {:?})",
                    target_fps, vis_style
                );

                while active_flag.load(Ordering::SeqCst) {
                    let loop_start = Instant::now();
                    let pcm = capture.read_latest(FFT_SIZE * 2);

                    analyzer.update(&pcm);
                    let t = start_time.elapsed().as_secs_f32();
                    let frame = renderer.render(&analyzer, t);

                    if let Err(e) = dev_clone.send_live_frame(&frame) {
                        warn!("Live frame send error: {}", e);
                    }

                    let elapsed = loop_start.elapsed();
                    if elapsed < period {
                        std::thread::sleep(period - elapsed);
                    }
                }

                info!("Music reactive stream thread exiting");
            });

            st.music_handle = Some(handle);

            DaemonResponse::Ok {
                message: format!(
                    "Music sync started (style: {}, FPS: {})",
                    vis_style.name(),
                    target_fps
                ),
            }
        }

        DaemonRequest::StopMusic => {
            st.stop_music();
            DaemonResponse::Ok {
                message: "Music visualization stopped".to_string(),
            }
        }

        DaemonRequest::SetCustomKey { key, color } => {
            let key_def = match find_key_by_name(&key) {
                Some(k) => k,
                None => {
                    return DaemonResponse::Error {
                        message: format!("Key '{}' not found in AK820 Max layout", key),
                    };
                }
            };

            let c = match Color::from_str(&color) {
                Ok(c) => c,
                Err(e) => {
                    return DaemonResponse::Error {
                        message: format!("Invalid color '{}': {}", color, e),
                    };
                }
            };

            st.custom_matrix[key_def.led_idx] = c;

            match st.ensure_device() {
                Ok(dev) => match dev.set_custom_matrix(&st.custom_matrix, 5) {
                    Ok(_) => DaemonResponse::Ok {
                        message: format!("Updated key {} to {}", key_def.label, c),
                    },
                    Err(e) => DaemonResponse::Error {
                        message: format!("Failed to upload custom matrix: {}", e),
                    },
                },
                Err(e) => DaemonResponse::Error {
                    message: format!("Device not connected: {}", e),
                },
            }
        }

        DaemonRequest::SetSleep { minutes } => match st.ensure_device() {
            Ok(dev) => match dev.set_sleep_timeout(minutes) {
                Ok(_) => DaemonResponse::Ok {
                    message: format!("Set sleep timeout to {} minutes", minutes),
                },
                Err(e) => DaemonResponse::Error {
                    message: format!("Failed to set sleep timeout: {}", e),
                },
            },
            Err(e) => DaemonResponse::Error {
                message: format!("Device not connected: {}", e),
            },
        },

        DaemonRequest::Shutdown => {
            st.stop_music();
            DaemonResponse::Ok {
                message: "Daemon shutting down".to_string(),
            }
        }
    }
}
