mod ipc;

use ajazz_audio::capture::AudioSource;
use ajazz_audio::visualizer::{ColorMode, ColorPalette, VisualizerConfig, VisualizerStyle};
use ajazz_audio::{AudioCapture, FFT_SIZE, MusicRenderer, SpectrumAnalyzer};
use ajazz_hid::{AjazzDevice, DeviceError, DeviceStatus, HardwareConnection, PowerState};
use ajazz_rgb::color::{Color, PaletteColor};
use ajazz_rgb::effects::{Direction, EffectMode, EffectSettings};
use ajazz_rgb::layout::{find_key_by_name, TOTAL_LEDS};
use ajazz_rgb::storage::PersistentConfig;
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
    music_hid_handle: Option<std::thread::JoinHandle<()>>,
    music_config: Arc<parking_lot::RwLock<VisualizerConfig>>,
    music_source: Arc<parking_lot::RwLock<AudioSource>>,
    music_action_trigger: Arc<std::sync::atomic::AtomicUsize>,
    custom_active: Arc<AtomicBool>,
    custom_hid_handle: Option<std::thread::JoinHandle<()>>,
    custom_matrix: Arc<parking_lot::RwLock<[Color; TOTAL_LEDS]>>,
    custom_brightness: Arc<std::sync::atomic::AtomicU8>,
    custom_notify: Arc<parking_lot::Condvar>,
    custom_lock: Arc<parking_lot::Mutex<()>>,
    last_settings: EffectSettings,
    persistent_config: PersistentConfig,
}

impl DaemonState {
    fn new(preferred_mode: Option<HardwareConnection>) -> Self {
        let cfg = PersistentConfig::load();
        let last_settings = EffectSettings::from(cfg.lighting.clone());
        let mut custom_mat = [Color::BLACK; TOTAL_LEDS];
        for (i, hex) in cfg.canvas.colors.iter().enumerate() {
            if i < TOTAL_LEDS {
                if let Ok(c) = Color::from_str(hex) {
                    custom_mat[i] = c;
                }
            }
        }
        let custom_bri = cfg.canvas.brightness;

        Self {
            device: None,
            preferred_mode,
            devnode_override: None,
            music_active: Arc::new(AtomicBool::new(false)),
            music_handle: None,
            music_hid_handle: None,
            music_config: Arc::new(parking_lot::RwLock::new(VisualizerConfig::default())),
            music_source: Arc::new(parking_lot::RwLock::new(AudioSource::SystemLoopback)),
            music_action_trigger: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            custom_active: Arc::new(AtomicBool::new(false)),
            custom_hid_handle: None,
            custom_matrix: Arc::new(parking_lot::RwLock::new(custom_mat)),
            custom_brightness: Arc::new(std::sync::atomic::AtomicU8::new(custom_bri)),
            custom_notify: Arc::new(parking_lot::Condvar::new()),
            custom_lock: Arc::new(parking_lot::Mutex::new(())),
            last_settings,
            persistent_config: cfg,
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
            if let Some(hid_handle) = self.music_hid_handle.take() {
                let _ = hid_handle.join();
            }
        }
    }

    fn stop_custom(&mut self) {
        if self.custom_active.swap(false, Ordering::SeqCst) {
            info!("Stopping background custom matrix live stream");
            self.custom_notify.notify_all();
            if let Some(h) = self.custom_hid_handle.take() {
                let _ = h.join();
            }
        }
    }

    fn stop_all_streaming(&mut self) {
        self.stop_music();
        self.stop_custom();
    }

    fn start_custom_stream(&mut self) {
        if self.custom_active.load(Ordering::SeqCst) {
            return;
        }
        self.stop_music();

        let dev = match self.ensure_device() {
            Ok(d) => d,
            Err(e) => {
                warn!("Cannot start custom stream without active device: {}", e);
                return;
            }
        };

        if let Err(e) = dev.enter_live_music_mode() {
            warn!("Warning initializing live frame mode for custom canvas: {}", e);
        }

        self.custom_active.store(true, Ordering::SeqCst);
        let active = Arc::clone(&self.custom_active);
        let matrix_arc = Arc::clone(&self.custom_matrix);
        let bri_arc = Arc::clone(&self.custom_brightness);
        let notify = Arc::clone(&self.custom_notify);
        let lock = Arc::clone(&self.custom_lock);
        let dev_clone = Arc::clone(&dev);

        let handle = std::thread::spawn(move || {
            info!("Daemon Custom Canvas live stream thread active (30 FPS)");
            while active.load(Ordering::SeqCst) {
                let frame = {
                    let m = matrix_arc.read();
                    let bri = bri_arc.load(Ordering::Relaxed).clamp(1, 5) as f32 / 5.0;
                    let mut scaled = [Color::BLACK; TOTAL_LEDS];
                    for i in 0..TOTAL_LEDS {
                        scaled[i] = m[i].scale(bri);
                    }
                    scaled
                };

                let _ = dev_clone.send_live_frame(&frame);

                // Wait ~30ms or wake up immediately if frame updated
                let mut guard = lock.lock();
                notify.wait_for(&mut guard, Duration::from_millis(30));
            }
            info!("Daemon Custom Canvas live stream thread stopped cleanly");
        });

        self.custom_hid_handle = Some(handle);
    }

    fn apply_saved_state(&mut self) {
        let cat = self.persistent_config.active_category.clone();
        if cat == "canvas" || self.persistent_config.canvas.is_active {
            info!("Restoring saved Custom Canvas lighting on hardware...");
            {
                let mut m = self.custom_matrix.write();
                for (i, hex) in self.persistent_config.canvas.colors.iter().enumerate() {
                    if i < TOTAL_LEDS {
                        if let Ok(c) = Color::from_str(hex) {
                            m[i] = c;
                        }
                    }
                }
            }
            self.custom_brightness.store(self.persistent_config.canvas.brightness, Ordering::Relaxed);
            self.start_custom_stream();
        } else if cat == "music" {
            // Apply fallback lighting effect until visualizer is explicitly active
            let settings = EffectSettings::from(self.persistent_config.lighting.clone());
            if let Ok(dev) = self.ensure_device() {
                let _ = dev.set_lighting(&settings);
            }
        } else {
            let settings = EffectSettings::from(self.persistent_config.lighting.clone());
            self.last_settings = settings.clone();
            if let Ok(dev) = self.ensure_device() {
                let _ = dev.set_lighting(&settings);
            }
        }

        // Apply saved sleep timeout
        let sleep_mins = self.persistent_config.device.sleep_minutes;
        if let Ok(dev) = self.ensure_device() {
            let _ = dev.set_sleep_timeout(sleep_mins);
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
            Ok(dev) => {
                info!(
                    "Connected to device: {} ({:?})",
                    dev.info().device_name,
                    dev.connection_mode()
                );
                st.apply_saved_state();
            }
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
    st.stop_all_streaming();
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

        DaemonRequest::GetStatus => {
            if st.music_active.load(Ordering::SeqCst) || st.custom_active.load(Ordering::SeqCst) {
                let cached = DeviceStatus {
                    battery_percentage: Some(100),
                    power_state: PowerState::FullyCharged,
                    is_charging: true,
                    raw: vec![],
                };
                return DaemonResponse::Status(cached);
            }
            match st.ensure_device() {
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
            }
        }

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
            st.stop_all_streaming();
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
            st.persistent_config.active_category = "effect".to_string();
            st.persistent_config.canvas.is_active = false;
            st.persistent_config.lighting = settings.clone().into();
            let _ = st.persistent_config.save();

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
            st.stop_all_streaming();
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
            st.persistent_config.active_category = "effect".to_string();
            st.persistent_config.canvas.is_active = false;
            st.persistent_config.lighting = settings.clone().into();
            let _ = st.persistent_config.save();

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
            palette,
            color_mode,
            sensitivity,
            noise_gate,
            decay,
            peak_hold,
            source,
            fps,
            morph_speed,
        } => {
            let vis_style = VisualizerStyle::from_name(&style).unwrap_or(VisualizerStyle::Spectrum);
            let parsed_color_mode = if let Some(ref cm_str) = color_mode {
                ColorMode::from_name(cm_str).unwrap_or(ColorMode::Palette(ColorPalette::StockClassic))
            } else if let Some(ref p_str) = palette {
                ColorMode::Palette(ColorPalette::from_name(p_str).unwrap_or(ColorPalette::StockClassic))
            } else if let Some(ref c_str) = color {
                if let Ok(c) = Color::from_str(c_str) {
                    ColorMode::CustomGradient {
                        base: c.scale(0.3),
                        peak: c,
                    }
                } else {
                    ColorMode::Palette(ColorPalette::StockClassic)
                }
            } else {
                ColorMode::Palette(ColorPalette::StockClassic)
            };

            let sens = sensitivity.unwrap_or(1.2).clamp(0.1, 5.0);
            let gate = noise_gate.unwrap_or(0.04).clamp(0.0, 0.4);
            let decay_spd = decay.unwrap_or(2.8).clamp(1.0, 6.0);
            let pk_hold = peak_hold.unwrap_or(true);
            let target_fps = fps.unwrap_or(60).clamp(10, 90);
            let morph = morph_speed.unwrap_or(1.5).clamp(0.1, 5.0);

            let audio_src = match source.as_deref() {
                Some("mic") | Some("microphone") => AudioSource::Microphone,
                Some("system") | Some("loopback") | Some("desktop") => AudioSource::SystemLoopback,
                Some(name) => AudioSource::Custom(name.to_string()),
                None => AudioSource::SystemLoopback,
            };

            // Save persistent config
            st.persistent_config.active_category = "music".to_string();
            st.persistent_config.canvas.is_active = false;
            st.persistent_config.music.style = style.clone();
            if let Some(ref cm) = color_mode {
                st.persistent_config.music.color_mode = cm.clone();
            }
            st.persistent_config.music.sensitivity = sens;
            st.persistent_config.music.noise_gate = gate;
            st.persistent_config.music.decay_speed = decay_spd;
            st.persistent_config.music.peak_hold = pk_hold;
            st.persistent_config.music.target_fps = target_fps;
            st.persistent_config.music.morph_speed = morph;
            st.persistent_config.music.audio_source = audio_src.name().to_string();
            let _ = st.persistent_config.save();

            // HOT-RELOAD CHECK: If already active and source is identical, update config in-place immediately without stopping!
            if st.music_active.load(Ordering::SeqCst) && *st.music_source.read() == audio_src {
                {
                    let mut cfg = st.music_config.write();
                    cfg.style = vis_style;
                    cfg.color_mode = parsed_color_mode.clone();
                    cfg.sensitivity = sens;
                    cfg.noise_gate = gate;
                    cfg.decay_speed = decay_spd;
                    cfg.peak_hold = pk_hold;
                    cfg.fps = target_fps;
                    cfg.morph_speed = morph;
                }
                return DaemonResponse::Ok {
                    message: format!("Instant hot-reload: style {:?}, mode {}", vis_style, parsed_color_mode.name()),
                };
            }

            // Otherwise, start or restart capture stream
            st.stop_all_streaming();
            *st.music_source.write() = audio_src.clone();
            {
                let mut cfg = st.music_config.write();
                cfg.style = vis_style;
                cfg.color_mode = parsed_color_mode;
                cfg.sensitivity = sens;
                cfg.noise_gate = gate;
                cfg.decay_speed = decay_spd;
                cfg.peak_hold = pk_hold;
                cfg.fps = target_fps;
                cfg.morph_speed = morph;
            }

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
            let active_flag_hid = Arc::clone(&st.music_active);
            let dev_clone = Arc::clone(&dev);
            let config_clone = Arc::clone(&st.music_config);
            let trigger_clone = Arc::clone(&st.music_action_trigger);

            // Single-slot conflated frame buffer: audio thread overwrites latest, HID writer grabs freshest
            let latest_frame: Arc<(parking_lot::Mutex<Option<[Color; TOTAL_LEDS]>>, parking_lot::Condvar)> =
                Arc::new((parking_lot::Mutex::new(None), parking_lot::Condvar::new()));
            let latest_frame_producer = Arc::clone(&latest_frame);
            let latest_frame_consumer = Arc::clone(&latest_frame);

            // Dedicated Zero-Latency Hardware HID Writer Thread
            let hid_handle = std::thread::spawn(move || {
                info!("Hardware HID Writer thread active (Latest-Frame-Wins)");
                while active_flag_hid.load(Ordering::SeqCst) {
                    let frame_opt = {
                        let (lock, cvar) = &*latest_frame_consumer;
                        let mut slot = lock.lock();
                        while slot.is_none() && active_flag_hid.load(Ordering::SeqCst) {
                            cvar.wait_for(&mut slot, Duration::from_millis(25));
                        }
                        slot.take() // Grab newest frame, drop any backlog!
                    };

                    if let Some(frame) = frame_opt {
                        let _ = dev_clone.send_live_frame(&frame);
                    }
                }
                info!("Hardware HID Writer thread stopped cleanly");
            });

            // Audio Capture & Visualizer Loop Thread
            let handle = std::thread::spawn(move || {
                info!("Daemon Audio Capture & Visualizer Engine started");
                let capture = match AudioCapture::new(Some(audio_src)) {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to initialize audio capture: {}", e);
                        active_flag.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                let init_cfg = config_clone.read().clone();
                let mut analyzer = SpectrumAnalyzer::new(capture.sample_rate(), init_cfg.sensitivity);
                analyzer.set_noise_gate(init_cfg.noise_gate);
                analyzer.set_decay_speed(init_cfg.decay_speed);
                analyzer.set_peak_hold(init_cfg.peak_hold);

                let mut renderer = MusicRenderer::with_config(init_cfg);

                let mut last_instant = Instant::now();

                while active_flag.load(Ordering::SeqCst) {
                    let start = Instant::now();
                    let dt = last_instant.elapsed().as_secs_f32().max(0.001);
                    last_instant = start;

                    // Sync in-memory hot-reloaded config (<1ms response!)
                    let target_fps = {
                        let cfg = config_clone.read();
                        analyzer.set_sensitivity(cfg.sensitivity);
                        analyzer.set_noise_gate(cfg.noise_gate);
                        analyzer.set_decay_speed(cfg.decay_speed);
                        analyzer.set_peak_hold(cfg.peak_hold);
                        renderer.config.style = cfg.style;
                        renderer.config.color_mode = cfg.color_mode.clone();
                        renderer.config.morph_speed = cfg.morph_speed;
                        cfg.fps
                    };

                    // Check action triggers
                    let action = trigger_clone.swap(0, Ordering::Relaxed);
                    if action == 1 {
                        renderer.shuffle_zone_gradient();
                    } else if action == 2 {
                        renderer.reroll_mosaic();
                    }

                    let pcm = capture.read_latest(FFT_SIZE);
                    if !pcm.is_empty() {
                        analyzer.update_with_dt(&pcm, dt);
                    }

                    let frame = renderer.render(&analyzer, dt);

                    // Overwrite single-slot buffer for HID writer thread
                    {
                        let (lock, cvar) = &*latest_frame_producer;
                        let mut slot = lock.lock();
                        *slot = Some(frame);
                        cvar.notify_one();
                    }

                    let frame_dur = Duration::from_micros((1_000_000 / target_fps as u64).max(11_111));
                    let elapsed = start.elapsed();
                    if elapsed < frame_dur {
                        std::thread::sleep(frame_dur - elapsed);
                    }
                }

                info!("Daemon Audio Capture thread exited");
            });

            st.music_handle = Some(handle);
            st.music_hid_handle = Some(hid_handle);
            DaemonResponse::Ok {
                message: format!("Started music visualization (style: {:?})", vis_style),
            }
        }

        DaemonRequest::StopMusic => {
            st.stop_music();
            let last = st.last_settings.clone();
            if let Ok(dev) = st.ensure_device() {
                let _ = dev.set_lighting(&last);
            }
            st.persistent_config.active_category = "effect".to_string();
            let _ = st.persistent_config.save();
            DaemonResponse::Ok {
                message: "Music visualization stopped".to_string(),
            }
        }

        DaemonRequest::ShuffleMusicZones => {
            st.music_action_trigger.store(1, Ordering::Relaxed);
            DaemonResponse::Ok {
                message: "Shuffled zone gradients".into(),
            }
        }

        DaemonRequest::RerollMusicMosaic => {
            st.music_action_trigger.store(2, Ordering::Relaxed);
            DaemonResponse::Ok {
                message: "Re-rolled static mosaic colors".into(),
            }
        }

        DaemonRequest::SetCustomKey { key, color } => {
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
                    {
                        let mut m = st.custom_matrix.write();
                        m[k.hw_idx] = c;
                    }
                    st.start_custom_stream();
                    st.custom_notify.notify_all();

                    st.persistent_config.active_category = "canvas".to_string();
                    st.persistent_config.canvas.is_active = true;
                    let current_colors: Vec<String> = st.custom_matrix.read().iter().map(|c| c.to_hex()).collect();
                    st.persistent_config.canvas.colors = current_colors;
                    let _ = st.persistent_config.save();

                    DaemonResponse::Ok {
                        message: format!("Set key '{}' to {}", k.label, c),
                    }
                }
                None => DaemonResponse::Error {
                    message: format!("Unknown key: '{}'", key),
                },
            }
        }

        DaemonRequest::SetCustomMatrix { colors, brightness } => {
            let bri = brightness.unwrap_or(5);
            st.custom_brightness.store(bri, Ordering::Relaxed);
            {
                let mut m = st.custom_matrix.write();
                for (i, c_str) in colors.iter().enumerate() {
                    if i < TOTAL_LEDS {
                        if let Ok(c) = Color::from_str(c_str) {
                            m[i] = c;
                        }
                    }
                }
            }
            st.start_custom_stream();
            st.custom_notify.notify_all();

            st.persistent_config.active_category = "canvas".to_string();
            st.persistent_config.canvas.is_active = true;
            st.persistent_config.canvas.brightness = bri;
            st.persistent_config.canvas.colors = colors;
            let _ = st.persistent_config.save();

            DaemonResponse::Ok {
                message: "Custom matrix saved and applied to hardware".to_string(),
            }
        }

        DaemonRequest::UpdateCustomLive { colors, brightness } => {
            let bri = brightness.unwrap_or(5);
            st.custom_brightness.store(bri, Ordering::Relaxed);
            {
                let mut m = st.custom_matrix.write();
                for (i, c_str) in colors.iter().enumerate() {
                    if i < TOTAL_LEDS {
                        if let Ok(c) = Color::from_str(c_str) {
                            m[i] = c;
                        }
                    }
                }
            }
            st.start_custom_stream();
            st.custom_notify.notify_all();

            st.persistent_config.active_category = "canvas".to_string();
            st.persistent_config.canvas.is_active = true;
            st.persistent_config.canvas.brightness = bri;
            st.persistent_config.canvas.colors = colors;
            let _ = st.persistent_config.save();

            DaemonResponse::Ok {
                message: "Live custom matrix updated (<1ms latency)".to_string(),
            }
        }

        DaemonRequest::GetConfig => {
            DaemonResponse::Config(st.persistent_config.clone())
        }

        DaemonRequest::SetSleep { minutes } => {
            st.persistent_config.device.sleep_minutes = minutes;
            let _ = st.persistent_config.save();
            match st.ensure_device() {
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
            }
        }

        DaemonRequest::Shutdown => {
            st.stop_all_streaming();
            std::process::exit(0);
        }
    }
}
