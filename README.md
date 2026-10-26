# Ajazz AK820 Max — Linux Native Driver, GUI Studio & RGB Music Visualizer

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Version: 1.1.1](https://img.shields.io/badge/Version-v1.1.1-brightgreen.svg)](Cargo.toml)
[![NixOS](https://img.shields.io/badge/NixOS-Flake%20Ready-5277C3.svg?logo=nixos&logoColor=white)](flake.nix)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-DEA584.svg?logo=rust&logoColor=white)](Cargo.toml)

A complete, high-performance, Rust-native Linux userspace driver, graphical studio (`ak820-gui`), command-line utility (`ak820ctl`), background daemon (`ak820d`), and real-time audio-reactive music visualizer for the **Ajazz AK820 Max** mechanical keyboard.

Built natively for **Linux & NixOS**, this project serves as a full, drop-in replacement for the vendor's Windows-only utility (`ajaz_ak820max.exe`), supporting both **BY-COMBO / SinoWealth** and **Shinetek / EVision** hardware controller revisions in both **2.4GHz Wireless** and **USB-C Wired** modes.

---

## What's New in v1.1.1

- 🔬 **1:1 Vendor Binary Reverse-Engineered Driver (`OemDrv.exe` & `CDevG5KB`)**:
  - Replaced all heuristic framing with the exact vendor binary communication protocol decompiled from `OemDrv.exe` (`0x0048abe0` `AccessData`, `0x0048b2d0` `SetLED`, `0x0048b970` `SetLedRgbTab`, `0x0048ae20` `SetCfg`).
  - Strict 520-byte Report ID 6 Feature Reports (`HIDIOCSFEATURE`/`HIDIOCGFEATURE`) with the required `0x5A 0xA5` magic payload trailer.
  - Resolved `No such device` (`ENODEV`, error 19) and stale device node caching with dynamic re-probing and strict non-swallowing error propagation.
  - Full hardware verification on real hardware (`258A:010C` Wired & `3554:FA09` Wireless): lighting effects, static color, per-key RGB canvas, audio-reactive visualizer streaming, and sleep timer.
- 🎨 **Native Linux GUI Application (`ak820-gui`)**:
  - Built with modern, hardware-accelerated **egui / eframe** running smoothly on both **Wayland** and **X11**.
  - **Interactive 75% (82-Key) Visual Canvas**: Proportional, physical layout matching the keyboard, with live color previews, per-key custom painting, click selection, drag marquee box, and quick presets (`WASD`, `Arrows`, `Function Row`, `Numbers`, `Alphas`, `Modifiers`, `All Keys`).
  - **Live Audio Visualizer Studio**: Real-time 10-band FFT spectrum analyzer (40 Hz – 16 kHz) with auto-gain control (AGC), pink noise tilt compensation, and peak decay animation.
  - **Lighting Effects Studio**: Browser for all 20 built-in hardware animations, brightness and speed sliders, direction toggle, HSV/Hex color picker, and 16 curated neon swatches.
  - **Hardware & Battery Header**: Live battery percentage, charging indicator, connection status (2.4G Wireless vs. Wired), active protocol, and interactive wireless sleep timer (5m, 15m, 30m, Never).
- 🖼️ **Desktop Environment Integration**:
  - Original 512x512 vector icon (`assets/icons/ajazz-ak820-max.svg`) and multi-resolution PNG icons (48x48 up to 512x512).
  - Freedesktop `.desktop` application menu entry with desktop actions for immediate access to Effects and Music modes.
- 📦 **First-Class Native Distro Packaging & Updaters**:
  - **NixOS Flake & Module**: One-command `nix run .#gui` launch, automated runtime library wrapping (`wayland`, `libxkbcommon`, `libGL`, `libX11`, `alsa-lib`), and seamless upgrades via `nix flake update && sudo nixos-rebuild switch`.
  - **Arch Linux (`PKGBUILD`)**: Clean standard package for `makepkg -si`.
  - **Debian / Ubuntu**: Debian package configuration and udev reload postinst hooks.
  - **Fedora / RHEL / openSUSE (`.spec`)**: RPM spec file for native RPM generation.
  - **Universal Makefile & Updater**: `make install`, `make update`, and `make uninstall` scripts for any Linux distribution.

---

## Features

- **Hardware Reverse-Engineered Support**:
  - **BY-COMBO / SinoWealth Revision** (`258A:010C` Wired & `3554:FA09` 2.4GHz Wireless): Uses 20-byte framed reports, modulo-256 checksums, and HID feature reports.
  - **Shinetek / EVision Revision** (`1A2C:A036` Wired & `1A2C:8FFF` 2.4GHz Wireless): Uses 64-byte `AA 55 CC 33` framed reports.
  - Automatic detection and seamless multi-interface protocol switching.
- **Real-Time Music Synchronization**:
  - Live audio capture via **CPAL** (PipeWire, PulseAudio, and ALSA).
  - High-resolution real-time FFT spectrum analysis with pink-noise tilt and auto-gain control (AGC).
  - **10 Frequency Equalizer Columns**: Physical mapping from the vendor's layout (`audiobar.txt`) across the 82 physical keys.
  - 5 Reactive styles: `spectrum` (rising bars), `pulse` (bass beat), `wave` (expanding ripples), `soft-dance` (flowing gradient), and `passion` (rock transients).
- **Comprehensive RGB Control**:
  - 20+ hardware effects: Static, Breathing, Rainbow Wave, Starry, Ripple, Ejection, Kaleidoscope, Raindrops, Reactive Typing, Windmill, etc.
  - Arbitrary per-key RGB coloring (WASD, arrows, custom key matrices).
  - Full control over brightness (1–5), speed (1–5), and direction.
- **Client/Daemon Architecture**:
  - `ak820-gui`: Responsive native Linux GUI studio.
  - `ak820d`: Background user daemon managing auto-reconnect, hotplugging, and background audio visualization.
  - `ak820ctl`: Fast CLI utility with colorful terminal output, status queries, and interactive music mode.

---

## Supported Hardware

| Revision | Mode | USB VID:PID | Device String | Protocol Variant |
|---|---|---|---|---|
| **BY-COMBO** | Wired USB | `258A:010C` | "BY Tech Gaming Keyboard" | `ByComboWired` (Feature Reports) |
| **BY-COMBO** | 2.4GHz Wireless | `3554:FA09` | "Compx 2.4G Wireless Receiver" | `ByComboWireless` (20-byte framing) |
| **Shinetek** | Wired USB | `1A2C:A036` | "Shinetek Technology AK820" | `ShinetekEvision` (64-byte framing) |
| **Shinetek** | 2.4GHz Wireless | `1A2C:8FFF` | "2.4 Wireless Device" | `ShinetekEvision` (64-byte framing) |

---

## Installation & Quick Start

### 1. Launch Instantly with Nix Flakes

No pre-installation required:

```bash
# Launch the Native GUI Studio
nix run .#gui

# Or query keyboard battery & status via CLI
nix run .#ak820ctl -- status

# Start CLI music visualizer directly
nix run .#ak820ctl -- music --style spectrum
```

### 2. NixOS Configuration Module

Add `ajazz-ak820-max` to your system configuration:

```nix
{
  inputs = {
    ajazz-ak820-max.url = "github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER";
  };

  outputs = { self, nixpkgs, ajazz-ak820-max, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      modules = [
        ajazz-ak820-max.nixosModules.default
        {
          # Enable hardware driver, udev rules, desktop app, and icons
          hardware.ajazz-ak820-max.enable = true;
          
          # Optional: automatically run daemon as a user systemd service
          hardware.ajazz-ak820-max.daemon.enable = true;
        }
      ];
    };
  };
}
```

To update to future releases on NixOS:
```bash
nix flake update ajazz-ak820-max
sudo nixos-rebuild switch
```

---

### 3. Native Linux Installation (Debian, Ubuntu, Arch, Fedora, etc.)

#### Universal Installer / Makefile:

```bash
git clone https://github.com/AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER.git
cd AJAZZ_AK820MAX_LINUX_DRIVER

# Build and install binaries, udev rules, systemd service, desktop entry & icons
sudo make install

# To update to latest upstream changes anytime:
sudo make update

# To uninstall cleanly:
sudo make uninstall
```

#### Arch Linux (PKGBUILD):

```bash
cd packaging/arch
makepkg -si
```

#### Fedora / RHEL / openSUSE (RPM):

```bash
rpmbuild -ba packaging/rpm/ajazz-ak820-max.spec
```

---

## GUI Studio Overview (`ak820-gui`)

The graphical interface is divided into 4 primary workspaces:

1. **✨ Lighting Effects Studio**:
   - Live virtual keyboard visualization reflecting active lighting animation.
   - Quick selection grid for all 20 hardware animations.
   - Sliders for brightness (1–5) and speed (1–5), plus left/right direction toggle.
   - Rainbow mode vs. custom color tint with 16 quick curated swatches and hex input.
2. **🎵 Audio-Reactive Visualizer**:
   - Real-time 10-band hardware FFT spectrum analyzer (40 Hz to 16 kHz).
   - Peak decay meter and animated decibel bars matching the physical key columns.
   - Instant style switcher (`spectrum`, `pulse`, `wave`, `soft-dance`, `passion`).
   - Sensitivity gain slider and target FPS selector (30, 60, 90 Hz).
3. **🎹 75% Visual Canvas & Custom Paint**:
   - Interactive, proportional 82-key layout canvas.
   - Click to toggle selection, or drag a marquee box across keys.
   - Preset buttons: `WASD`, `Arrows`, `Fn Row`, `Numbers`, `Alphas`, `Modifiers`, `All Keys`.
   - Paint selected keys with any color swatch and flash to keyboard memory.
4. **⚙️ Device & Power Settings**:
   - Live hardware status: Battery percentage gauge, charging/discharging state.
   - Connection mode: 2.4GHz Wireless vs. Wired USB.
   - Controller protocol identification and active devnode (`/dev/hidraw*`).
   - Interactive wireless sleep timeout control (5m, 15m, 30m, Never).

---

## CLI Usage Reference (`ak820ctl`)

```bash
# Query battery and connection status
ak820ctl status

# List all detected Ajazz keyboards and USB interfaces
ak820ctl devices

# Set built-in lighting effects
ak820ctl effect --mode breathing --brightness 5 --speed 4
ak820ctl effect --mode sine-wave --direction left
ak820ctl effect --mode spectrum

# Set static solid color
ak820ctl static --color "#00F2FE"

# Launch real-time music reactive lighting
ak820ctl music --style spectrum --sensitivity 1.5 --fps 60
ak820ctl music --style pulse --color "#FF007F"

# Set individual key color
ak820ctl key --name wasd --color "#00FF88"

# Set auto-sleep timer (in minutes, 0 = never)
ak820ctl sleep --minutes 15
```

---

## Running the Background Daemon (`ak820d`)

The background daemon enables hotplug recovery and keeps music visualization running smoothly:

```bash
# Start manually
ak820d

# Or enable as a systemd user service
systemctl --user enable --now ak820d.service
```

---

## Testing & Quality Assurance

Run the comprehensive unit and integration test suite:

```bash
cargo test --workspace
```

Tests include:
- Protocol framing and checksum verification for BY-COMBO and Shinetek revisions.
- Physical key layout geometry and 108 LED index bounds verification.
- Audio RealFFT spectrum analyzer logarithmic band segmentation and AGC validation.

---

## License & Credits

- **License**: Dual-licensed under [MIT](LICENSE) or [Apache 2.0](LICENSE-APACHE).
- **Author**: Aayam Pokhrel (<pokhrelaayam1010@gmail.com>).
- **Reverse Engineering**: Built using cleanroom protocol analysis and reverse engineering of USB/HID traffic and the vendor Windows utility.
