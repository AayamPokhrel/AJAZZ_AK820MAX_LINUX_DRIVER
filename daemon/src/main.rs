mod ipc;

use ajazz_audio::{AudioCapture, FFT_SIZE, MusicRenderer, SpectrumAnalyzer, VisualizerStyle};
use ajazz_hid::{AjazzDevice, DeviceError, HardwareConnection};
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

    #[arg(short, long, help = "Preferred connection mode: wired, 2.4g, bluetooth, auto")]
    mode: Option<String>,

    #[arg(short, long, help = "Enable verbose debug logging")]
    verbose: bool,
}

struct DaemonState {
    device: Option<Arc<AjazzDevice>>,
    preferred_mode: Option<HardwareConnection>,
    devnode_override: Option<String>,
    music_active: Arc<AtomicBool>,
    music_handle: Option<std::thread::JoinHandle<()>>,
    custom_matrix: [Color; TOTAL_LEDS],
    last_settings: EffectSettings,
}

impl DaemonState {
    fn new(preferred_mode: Option<HardwareConnection>) -> Self {
        Self {
            device: None,
            preferred_mode,
            devnode_override: None,
            music_active: Arc::new(AtomicBool::new(false)),
            music_handle: None,
            custom_matrix: [Color::BLACK; TOTAL_LEDS],
            last_settings: EffectSettings::default(),
        }
    }

    fn ensure_device(&mut self) -> Result<Arc<AjazzDevice>, DeviceError> {
        // If device exists, check if still alive
        if let Some(ref dev) = self.device {
            if dev.status().is_ok() {
                return Ok(Arc::clone(dev));
            }
            warn!("Current device link unresponsive; re-probing connection modes...");
            self.device = None;
        }

        let dev = Arc::new(AjazzDevice::open_with_preference(
            self.preferred_mode,
            self.devnode_override.as_deref(),
        )?);
        info!(
            "Active keyboard connected: {} ({}, mode: {:?})",
            dev.info().device_name,
            dev.info().devnode,
            dev.connection_mode()
        );
        self.device = Some(Arc::clone(&dev));
        Ok(dev)
    }

    fn set_mode_preference(&mut self, mode_str: &str, dev_override: Option<String>) {
        self.devnode_override = dev_override;
        self.preferred_mode = match mode_str.to_lowercase().as_str() {
            "auto" => None,
            s => HardwareConnection::from_str_case(s),
        };
        // Invalidate current device to reconnect under new preference
        self.device = None;
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

    let pref_mode = args.mode.as_deref().and_then(HardwareConnection::from_str_case);
    let state = Arc::new(Mutex::new(DaemonState::new(pref_mode)));

    // Try initial connection
    {
        let mut st = state.lock().await;
        match st.ensure_device() {
            Ok(dev) => info!(
                "Connected to device: {} ({:?})",
                dev.info().device_name,
                dev.connection_mode()
            ),
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
                Err(e) => {
                    st.device = None;
                    DaemonResponse::Error {
                        message: format!("Failed to read status: {}", e),
                    }
                }
            },
            Err(e) => DaemonResponse::Error {
                message: format!("Device not connected: {}", e),
            },
        },

        DaemonRequest::GetDevices => {
            let devs = ajazz_hid::enumerate_devices();
            DaemonResponse::Devices(devs)
        }

        DaemonRequest::SetMode { mode, device } => {
            st.set_mode_preference(&mode, device);
            match st.ensure_device() {
                Ok(dev) => DaemonResponse::Ok {
                    message: format!(
                        "Switched to {} on {} (mode: {:?})",
                        dev.info().device_name,
                        dev.info().devnode,
                        dev.connection_mode()
                    ),
                },
                Err(e) => DaemonResponse::Error {
                    message: format!("Failed to connect under mode '{}': {}", mode, e),
                },
            }
        }

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
                    Err(e) => {
                        st.device = None;
                        DaemonResponse::Error {
                            message: format!("Failed to set effect: {}", e),
                        }
                    }
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
                    Err(e) => {
                        st.device = None;
                        DaemonResponse::Error {
                            message: format!("Failed to set static color: {}", e),
                        }
                    }
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

            let custom_tint = color.and_then(|c| Color::from_str(&c).ok());
            let target_fps = fps.unwrap_or(30).clamp(10, 60);
            let sens = sensitivity.unwrap_or(1.0).clamp(0.1, 5.0);

            let dev = match st.ensure_device() {
                Ok(d) => d,
                Err(e) => {
                    return DaemonResponse::Error {
                        message: format!("Device unavailable: {}", e),
                    };
                }
            };

            if let Err(e) = dev.enter_live_music_mode() {
                warn!("Device live mode initialization warning: {}", e);
            }

            st.music_active.store(true, Ordering::SeqCst);
            let active_flag = Arc::clone(&st.music_active);
            let dev_clone = Arc::clone(&dev);

            let handle = std::thread::spawn(move || {
                info!(
                    "Music visualizer thread started (style: {:?}, target {} FPS, sensitivity {:.2})",
                    vis_style, target_fps, sens
                );

                let capture = match AudioCapture::new(None) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to initialize audio capture: {}", e);
                        active_flag.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), sens);
                let mut renderer = MusicRenderer::new(vis_style, custom_tint);

                let frame_dur = Duration::from_micros((1_000_000 / target_fps as u64).max(16_666));

                while active_flag.load(Ordering::SeqCst) {
                    let start = Instant::now();
                    let pcm = capture.read_latest(FFT_SIZE);
                    if !pcm.is_empty() {
                        analyzer.update(&pcm);
                    }

                    let frame = renderer.render(&analyzer, 0.033);
                    let _ = dev_clone.send_live_frame(&frame);

                    let elapsed = start.elapsed();
                    if elapsed < frame_dur {
                        std::thread::sleep(frame_dur - elapsed);
                    }
                }

                info!("Music visualizer thread exited");
            });

            st.music_handle = Some(handle);
            DaemonResponse::Ok {
                message: format!("Started music visualization (style: {:?})", vis_style),
            }
        }

        DaemonRequest::StopMusic => {
            st.stop_music();
            DaemonResponse::Ok {
                message: "Music visualization stopped".to_string(),
            }
        }

        DaemonRequest::SetCustomKey { key, color } => {
            st.stop_music();
            let c = match Color::from_str(&color) {
                Ok(c) => c,
                Err(e) => {
                    return DaemonResponse::Error {
                        message: format!("Invalid color: {}", e),
                    };
                }
            };

            let key_opt = find_key_by_name(&key);
            match key_opt {
                Some(k) => {
                    st.custom_matrix[k.led_idx] = c;
                    let matrix = st.custom_matrix;
                    match st.ensure_device() {
                        Ok(dev) => match dev.set_custom_matrix(&matrix, 5) {
                            Ok(_) => DaemonResponse::Ok {
                                message: format!("Set key '{}' to {}", k.label, c),
                            },
                            Err(e) => {
                                st.device = None;
                                DaemonResponse::Error {
                                    message: format!("Failed to upload custom matrix: {}", e),
                                }
                            }
                        },
                        Err(e) => DaemonResponse::Error {
                            message: format!("Device not connected: {}", e),
                        },
                    }
                }
                None => DaemonResponse::Error {
                    message: format!("Unknown key: '{}'", key),
                },
            }
        }

        DaemonRequest::SetCustomMatrix { colors, brightness } => {
            st.stop_music();
            let bri = brightness.unwrap_or(5);
            for (i, c_str) in colors.iter().enumerate() {
                if i < TOTAL_LEDS {
                    if let Ok(c) = Color::from_str(c_str) {
                        st.custom_matrix[i] = c;
                    }
                }
            }
            let matrix = st.custom_matrix;
            match st.ensure_device() {
                Ok(dev) => match dev.set_custom_matrix(&matrix, bri) {
                    Ok(_) => DaemonResponse::Ok {
                        message: "Custom matrix uploaded successfully".to_string(),
                    },
                    Err(e) => {
                        st.device = None;
                        DaemonResponse::Error {
                            message: format!("Failed to upload custom matrix: {}", e),
                        }
                    }
                },
                Err(e) => DaemonResponse::Error {
                    message: format!("Device not connected: {}", e),
                },
            }
        }

        DaemonRequest::SetSleep { minutes } => match st.ensure_device() {
            Ok(dev) => match dev.set_sleep_timeout(minutes) {
                Ok(_) => DaemonResponse::Ok {
                    message: format!("Set sleep timeout to {} min", minutes),
                },
                Err(e) => {
                    st.device = None;
                    DaemonResponse::Error {
                        message: format!("Failed to set sleep timeout: {}", e),
                    }
                }
            },
            Err(e) => DaemonResponse::Error {
                message: format!("Device not connected: {}", e),
            },
        },

        DaemonRequest::Shutdown => {
            st.stop_music();
            std::process::exit(0);
        }
    }
}
