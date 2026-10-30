# Ajazz AK820 Max — Linux User Guide (v1.2.0)

This comprehensive guide covers using, configuring, building, and packaging the native Linux software suite for the **Ajazz AK820 Max** mechanical keyboard.

---

## Table of Contents

1. [Software Suite Overview](#1-software-suite-overview)
2. [Native GUI Studio (`ak820-gui`)](#2-native-gui-studio-ak820-gui)
3. [CLI Reference (`ak820ctl`)](#3-cli-reference-ak820ctl)
4. [Background Daemon (`ak820d`)](#4-background-daemon-ak820d)
5. [Installation & Upgrade Workflows](#5-installation--upgrade-workflows)
   - [NixOS Flakes & Module](#51-nixos-flakes--module)
   - [Universal Makefile & Installer](#52-universal-makefile--installer)
   - [Arch Linux (PKGBUILD)](#53-arch-linux-pkgbuild)
   - [Debian / Ubuntu](#54-debian--ubuntu)
   - [Fedora / RHEL / openSUSE (RPM)](#55-fedora--rhel--opensuse-rpm)
6. [Troubleshooting & FAQ](#6-troubleshooting--faq)

---

## 1. Software Suite Overview

The Ajazz AK820 Max Linux suite consists of three tightly-integrated components:

1. **`ak820-gui`**: A graphical studio written in Rust using `egui` and `eframe`. Provides an interactive 75% 82-key visual canvas, per-key RGB custom painting, a 20-effect lighting studio, a real-time 10-band audio FFT spectrum analyzer, and battery/power hardware controls.
2. **`ak820ctl`**: A command-line control tool for fast status queries, shell scripts, hotkeys, and headless terminal usage.
3. **`ak820d`**: A background user daemon communicating over a Unix domain socket (`/run/user/$UID/ajazz-ak820.sock`), handling hotplug auto-reconnection and continuous music visualization without requiring open terminals.

---

## 2. Native GUI Studio (`ak820-gui`)

Launch the GUI from your desktop application launcher or via command line:

```bash
ak820-gui

# Or specify an initial tab:
ak820-gui --tab music
ak820-gui --tab effects
ak820-gui --tab canvas
ak820-gui --tab hardware
```

### 2.1 Top Status Header
- **Live Battery Indicator**: Shows battery percentage, discharging/charging power status, and color-coded gauge (Green ≥ 50%, Amber 20–50%, Red < 20%).
- **Connection Mode**: Displays whether the GUI is communicating via `ak820d IPC` or `Direct HID`.
- **Physical Link**: Detects whether the keyboard is connected via **2.4GHz Wireless** (USB Dongle) or **USB-C Wired**.
- **⟳ Re-scan**: Forces an immediate re-probe of device nodes and re-connects to the daemon.

### 2.2 ✨ Lighting Effects Studio
- **20 Built-in Animations**: One-click switching between `Static`, `Breathing`, `Sine Wave`, `Spectrum`, `Ripple L-R`, `Ripple U-D`, `Starry`, `Ejection`, `Kaleidoscope`, `Cross`, `Reactive`, `Scroll`, `Top-Bottom`, `Rain`, `Windmill`, `Galloping`, `Music`, `Screen`, `Custom`, and `Lights Off`.
- **Brightness & Speed**: Discrete 1 to 5 level sliders with real-time hardware synchronization.
- **Direction**: Left / Right animation direction toggle.
- **Color Selector**: Toggle between Rainbow gradient animation and custom solid color with 16 curated neon swatches and Hex color code input.

### 2.3 🎵 Audio-Reactive Visualizer Studio
- **10-Band Real-Time Frequency Spectrum Analyzer**: Renders audio capture from PipeWire / PulseAudio / ALSA spanning 40 Hz to 16 kHz with floating peak decay indicators and logarithmic decibel scaling.
- **Visualizer Styles**:
  - `Equalizer Spectrum`: Columns rise from bottom to top across the 10 keyboard frequency bands.
  - `Bass Pulse`: Pulsates keyboard backlight intensity to the rhythmic low-end kick.
  - `Ripple Wave`: Expanding acoustic waves radiating outward from the keyboard center.
  - `Soft Dance`: Harmonized melodic gradient flowing across key rows.
  - `Passion Rock`: High-intensity transient flash bursts on sharp snare and percussion hits.
- **Sensitivity & FPS Controls**: Adjustable gain multiplier (0.2x to 4.0x) and target refresh rate (30, 60, 90 Hz).

### 2.4 🎹 75% Visual Canvas & Custom Paint
- Proportional 82-key AK820 Max physical layout canvas.
- **Quick Selection Presets**: `WASD`, `Arrows`, `Fn Row`, `Numbers`, `Alphas`, `Modifiers`, `All Keys`, `Clear`.
- **Marquee Drag Selection**: Click and drag across the canvas to select custom groups of keys.
- **Paint Palette**: Click any swatch to apply that color to all selected keys.
- **Save to Hardware**: Uploads the custom painted matrix directly to the keyboard's non-volatile onboard profile.

### 2.5 ⚙️ Device & Power Configuration
- Displays USB Vendor ID, Product ID, devnode path (`/dev/hidraw*`), and hardware protocol revision.
- **Wireless Auto-Sleep Timer**: Configure the controller sleep timeout to `5 min`, `15 min`, `30 min`, or `Never (Always On)`.

---

## 3. CLI Reference (`ak820ctl`)

```bash
# Query status and battery
ak820ctl status

# List detected devices
ak820ctl devices

# Set effects
ak820ctl effect --mode breathing --brightness 5 --speed 4
ak820ctl effect --mode sine-wave --direction left
ak820ctl effect --mode spectrum

# Set solid static color
ak820ctl static --color "#00F2FE"

# Launch music sync
ak820ctl music --style spectrum --sensitivity 1.2 --fps 60

# Paint specific keys
ak820ctl key --name wasd --color "#00FF88"
ak820ctl key --name space --color "#A855F7"

# Set sleep timeout (0 = never)
ak820ctl sleep --minutes 15
```

---

## 4. Background Daemon (`ak820d`)

The daemon runs silently in the background, listening on `/run/user/$UID/ajazz-ak820.sock`.

```bash
# Start manually:
ak820d

# Enable and start via systemd user unit:
systemctl --user enable --now ak820d.service

# Check daemon logs:
journalctl --user -u ak820d.service -f
```

---

## 5. Installation & Upgrade Workflows

### 5.1 NixOS Flakes & Module

#### Direct execution with zero installation:
```bash
nix run github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER#gui
```

#### System configuration (`configuration.nix` or `flake.nix`):
```nix
{
  inputs.ajazz-ak820-max.url = "github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER";

  outputs = { self, nixpkgs, ajazz-ak820-max, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      modules = [
        ajazz-ak820-max.nixosModules.default
        {
          hardware.ajazz-ak820-max.enable = true;
          hardware.ajazz-ak820-max.daemon.enable = true;
        }
      ];
    };
  };
}
```

#### Updating on NixOS:
```bash
nix flake update ajazz-ak820-max
sudo nixos-rebuild switch
```

---

### 5.2 Universal Makefile & Installer

Works on any Linux distribution with Rust or Nix installed:

```bash
git clone https://github.com/AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER.git
cd AJAZZ_AK820MAX_LINUX_DRIVER

# Install binaries, udev rules, systemd service, desktop entry & icons
sudo make install

# Update to latest git release and re-install atomically:
sudo make update

# Uninstall cleanly:
sudo make uninstall
```

---

### 5.3 Arch Linux (PKGBUILD)

```bash
cd packaging/arch
makepkg -si
```

---

### 5.4 Debian / Ubuntu

Install required build dependencies:
```bash
sudo apt-get install build-essential pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
```

Build and install via Makefile or generate `.deb`:
```bash
sudo make install
```

---

### 5.5 Fedora / RHEL / openSUSE (RPM)

Install RPM build tools:
```bash
sudo dnf install rpm-build cargo alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel
rpmbuild -ba packaging/rpm/ajazz-ak820-max.spec
```

---

## 6. Troubleshooting & FAQ

### Q: "Permission denied opening /dev/hidraw*"
Ensure the udev rule is installed and reloaded:
```bash
sudo cp nix/70-ajazz-ak820.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
```
Unplug and replug the USB dongle or cable. Verify your user belongs to the `input` group:
```bash
groups | grep input
```

### Q: "Audio reactive mode does not react to desktop sound"
Make sure your audio server (PipeWire or PulseAudio) has a monitor device. In PipeWire, `pavucontrol` can verify your output sink has an active monitor stream.
You can also increase the sensitivity gain slider in `ak820-gui` or pass `--sensitivity 2.0` in CLI.

### Q: "Does the GUI work on Wayland?"
Yes! `ak820-gui` uses hardware-accelerated OpenGL (`glow`) and native Wayland protocol client libraries (`wayland-client`, `libxkbcommon`). It runs natively under GNOME Wayland, KDE Plasma Wayland, Hyprland, Sway, and Cosmic Desktop.
