# Ajazz AK820 Max — Linux Userspace Driver and Control Suite

A native Linux userspace driver, graphical control studio (`ak820-gui`), command-line utility (`ak820ctl`), and background daemon (`ak820d`) for the Ajazz AK820 Max mechanical keyboard.

This software serves as a native replacement for the vendor's Windows-only configuration utility (`DeviceDriver.exe` / `OemDrv.exe`). It supports both the BY-COMBO / SinoWealth and Shinetek / EVision hardware revisions in wired and 2.4 GHz wireless modes.

---

## Supported Hardware

| Controller Revision | Connection Mode | USB VID:PID | Device Identification | Transport Protocol |
|---|---|---|---|---|
| BY-COMBO / SinoWealth | USB Wired | `258a:010c` | "BY Tech Gaming Keyboard" | 520-byte Feature Reports (Report ID 6) |
| BY-COMBO / SinoWealth | 2.4 GHz Wireless | `3554:fa09` | "Compx 2.4G Wireless Receiver" | 20-byte framed reports with mod-256 checksum |
| Shinetek / EVision | USB Wired | `1a2c:a036` | "Shinetek Technology AK820" | 64-byte framed reports (`AA 55 CC 33` magic) |
| Shinetek / EVision | 2.4 GHz Wireless | `1a2c:8fff` | "2.4 Wireless Device" | 64-byte framed reports (`AA 55 CC 33` magic) |

---

## Architecture

The project is structured as a modular Rust workspace:

- **`hid` (`ajazz-hid`)**: Hardware abstraction layer interacting directly with Linux `/dev/hidraw` nodes via `hidapi` and `rusb`. Implements automatic protocol negotiation, device probing, multi-interface filtering, and 1:1 vendor binary packet serialization.
- **`rgb` (`ajazz-rgb`)**: Lighting subsystem implementing the 20 built-in hardware animations, custom 82-key matrix coordinate mapping, spatial gradient rendering, and real-time typing-reactive effect generators.
- **`audio` (`ajazz-audio`)**: Audio capture and signal processing pipeline using `cpal` to capture system desktop audio output (PipeWire or PulseAudio monitor loopbacks). Implements a real-time FFT spectrum analyzer with pink-noise tilt compensation and automatic gain control (AGC), mapped to physical keyboard column frequencies.
- **`daemon` (`ak820d`)**: Background user service that monitors device hotplugging, manages sleep timers, captures non-blocking keyboard input events via `evdev` for typing-reactive animations, and exposes a Unix domain socket (`$XDG_RUNTIME_DIR/ajazz-ak820.sock`) for client IPC.
- **`cli` (`ak820ctl`)**: Terminal utility providing hardware status queries, lighting configuration, solid color assignment, sleep management, and terminal-based music visualization. Communicates with `ak820d` via IPC when running, with automatic fallback to direct HID access.
- **`gui` (`ak820-gui`)**: Hardware-accelerated graphical interface built with `egui` and `eframe`. Supports native Wayland and X11 display servers, providing an interactive 75% canvas editor, lighting browser, audio visualizer controls, and battery telemetry.

---

## Prerequisites

### Kernel and Permissions

The driver operates entirely in userspace and requires:
- Linux kernel compiled with `CONFIG_HIDRAW=y` and `CONFIG_INPUT_EVDEV=y`.
- Udev rules granting unprivileged read/write access to the keyboard's `/dev/hidraw*` devices.

A pre-configured udev rule file is provided at `nix/70-ajazz-ak820.rules` and installed by the setup scripts:

```udev
# BY-COMBO / SinoWealth Wired Mode (258a:010c)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="258a", ATTRS{idProduct}=="010c", MODE="0666", GROUP="input", TAG+="uaccess"

# Compx 2.4GHz Wireless USB Receiver (3554:fa09)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="3554", ATTRS{idProduct}=="fa09", MODE="0666", GROUP="input", TAG+="uaccess"

# Shinetek / EVision Wired Mode (1a2c:a036)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1a2c", ATTRS{idProduct}=="a036", MODE="0666", GROUP="input", TAG+="uaccess"

# Shinetek 2.4GHz Wireless USB Receiver (1a2c:8fff)
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="1a2c", ATTRS{idProduct}=="8fff", MODE="0666", GROUP="input", TAG+="uaccess"
```

Desktop users must be members of the `input` group to access evdev devices for typing-reactive lighting.

### Build Dependencies

Building from source requires the Rust toolchain (2021 edition) and standard system development headers:
- `pkg-config`
- `systemd` / `udev` (`libudev`)
- `alsa-lib` and `libpulseaudio`
- Graphical libraries: `wayland`, `libxkbcommon`, `libGL`, `vulkan-loader`, `libX11`, `libXcursor`, `libXrandr`, `libXi`

On Debian/Ubuntu-derived systems:
```bash
sudo apt-get install pkg-config libudev-dev libasound2-dev libpulse-dev \
    libwayland-dev libxkbcommon-dev libgl1-mesa-dev libvulkan-dev \
    libx11-dev libxcursor-dev libxrandr-dev libxi-dev
```

On Arch Linux:
```bash
sudo pacman -S base-devel pkgconf systemd alsa-lib libpulse \
    wayland libxkbcommon mesa vulkan-icd-loader libx11 libxcursor libxrandr libxi
```

On Fedora:
```bash
sudo dnf install pkgconf-pkg-config systemd-devel alsa-lib-devel pulseaudio-libs-devel \
    wayland-devel libxkbcommon-devel mesa-libGL-devel vulkan-loader-devel \
    libX11-devel libXcursor-devel libXrandr-devel libXi-devel
```

---

## Installation & Setup

### NixOS (Flake Module)

1. Add the repository to your system flake inputs in `/etc/nixos/flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    ajazz-ak820-max = {
      url = "github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, ajazz-ak820-max, ... }@inputs: {
    nixosConfigurations.nixos = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [
        ajazz-ak820-max.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

2. Enable the hardware module and background daemon in `/etc/nixos/configuration.nix`:

```nix
{
  # Add user to input group for reactive typing capture
  users.users."yourusername".extraGroups = [ "input" ];

  # Enable the Ajazz driver, udev rules, desktop entries, and daemon
  hardware.ajazz-ak820-max.enable = true;
  hardware.ajazz-ak820-max.daemon.enable = true;
}
```

3. Rebuild and switch:

```bash
sudo nixos-rebuild switch --flake /etc/nixos#nixos
```

To run individual binaries directly using Nix Flakes without installing system-wide:

```bash
# Launch GUI Studio
nix run github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER#gui

# Check hardware status
nix run github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER#ak820ctl -- status
```

### Generic Linux (Makefile)

1. Clone the repository:

```bash
git clone https://github.com/AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER.git
cd AJAZZ_AK820MAX_LINUX_DRIVER
```

2. Build release binaries:

```bash
make build
```

3. Install binaries, udev rules, desktop entries, icons, and systemd user services:

```bash
sudo make install
```

4. Reload udev rules and enable the background daemon for your user account:

```bash
sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=hidraw
systemctl --user daemon-reload
systemctl --user enable --now ak820d.service
```

To uninstall cleanly:

```bash
sudo make uninstall
```

### Distribution Packages

- **Arch Linux**: A `PKGBUILD` is provided in `packaging/arch/`. Run `makepkg -si` from that directory.
- **RPM (Fedora / RHEL / openSUSE)**: A spec file is provided in `packaging/rpm/ajazz-ak820-max.spec`.

---

## Usage

### Command-Line Interface (`ak820ctl`)

`ak820ctl` interacts with the running background daemon over Unix domain socket IPC, or opens the `/dev/hidraw*` endpoint directly if the daemon is stopped.

#### Status and Device Enumeration

```bash
# Query battery level, charging status, and connection mode
ak820ctl status

# List detected Ajazz USB devices, dongles, and hidraw paths
ak820ctl devices
```

#### Lighting Effects

```bash
# Set a hardware animation with speed (1-5) and brightness (1-5)
ak820ctl effect --mode breathing --brightness 5 --speed 3
ak820ctl effect --mode sine-wave --direction left
ak820ctl effect --mode spectrum

# Set a static RGB color across the keyboard
ak820ctl static --color "#00F2FE"

# Color specific key groups (e.g. wasd, arrows, numbers)
ak820ctl key --name wasd --color "#00FF88"
```

Supported hardware effect modes include: `static`, `breathing`, `spectrum`, `starry`, `ripple-lr`, `ripple-ud`, `flash-away`, `kaleidoscope`, `cross`, `reactive-single`, `picture-scroll`, `waterfall`, `raindrops`, `windmill`, `sine-wave`, `storm`, and interactive typing modes (`gradient-splash`, `neon-shockwave`, `matrix-rain`, `thermal-fire`).

#### Audio-Reactive Music Visualizer

```bash
# Start terminal-driven music visualizer with PipeWire/PulseAudio loopback
ak820ctl music --style spectrum --sensitivity 1.5 --fps 60

# Bass beat pulse mode with custom neon tint
ak820ctl music --style pulse --color "#FF007F"
```

#### Power Management

```bash
# Set wireless auto-sleep timeout in minutes (0 = never sleep)
ak820ctl sleep --minutes 15
```

### Graphical Interface (`ak820-gui`)

Launch `ak820-gui` from your application menu or terminal. The interface is organized into four main tabs:

1. **Lighting Effects Studio**: Select from all 20 hardware lighting animations, adjust brightness and playback speed sliders, toggle direction, and pick custom colors via HSV palette or hex code.
2. **Audio-Reactive Visualizer**: Configure real-time music synchronization with 10-band spectrum visualization, sensitivity controls, FPS targets, and reactive styles (`spectrum`, `pulse`, `wave`, `soft-dance`, `passion`).
3. **75% Visual Canvas**: An interactive 82-key graphical representation of the keyboard layout. Supports clicking individual keys, dragging marquee selection boxes, applying quick presets (`WASD`, `Arrows`, `Function Row`, `Modifiers`), and saving custom per-key layouts to hardware memory.
4. **Device & Power Settings**: Real-time display of battery percentage, charging state, active transport protocol, devnode assignment, and wireless sleep configuration.

### Background Daemon (`ak820d`)

The daemon manages hardware state, persistent configurations, audio synchronization, and input event capture without requiring root privileges.

```bash
# Check daemon service status
systemctl --user status ak820d.service

# View daemon logs
journalctl --user -u ak820d.service -f

# Restart daemon
systemctl --user restart ak820d.service
```

---

## Testing

Run the automated test suite across all workspace crates:

```bash
cargo test --workspace
```

The test suite validates:
- Modulo-256 and XOR checksum generation across both protocol families.
- 520-byte and 64-byte packet serialization and deserialization.
- Physical layout coordinate bounds and 108 LED index constraints.
- RealFFT logarithmic bin division, auto-gain normalization, and DSP decay curves.

---

## Credits & Acknowledgments

- **REA (Reverse Engineering Anything)**: Specifically acknowledged for protocol reconstruction, disassembly analysis, and structure recovery from the proprietary Windows binaries (`DeviceDriver.exe`, `OemDrv.exe`, and `CDevG5KB.dll`).
- **egui & eframe** by Emil Ernerfeldt: Immediate-mode graphical user interface library for Rust, powering `ak820-gui`.
- **CPAL (Cross-Platform Audio Library)**: Audio I/O capture infrastructure for PipeWire, PulseAudio, and ALSA loopbacks.
- **realfft**: High-performance real-to-complex Fast Fourier Transform implementation used in the audio visualizer pipeline.
- **hidapi / hidapi-rs**: Userspace HID transport library interface.
- **tokio**: Asynchronous runtime driving daemon IPC, event loops, and non-blocking I/O.
- **clap**: Declarative command-line argument parser for `ak820ctl`.
- **evdev**: Linux event device bindings utilized for zero-latency interactive typing animations.
- **Nix & NixOS Community**: Packaging infrastructure, systemd module conventions, and reproducible environment tooling.

---

## AI Disclosure

Portions of this project were developed with the assistance of agentic AI coding tools (Antigravity). AI assistance was utilized for exploratory protocol translation, scaffolding initial crate layouts, DSP mathematical algorithms, and Nix packaging configurations. All protocols, hardware communications, and functionality were reverse-engineered, reviewed, and validated against physical Ajazz AK820 Max hardware.
