# Ajazz AK820 Max — Linux Native Userspace Driver & RGB Music Visualizer

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![NixOS](https://img.shields.io/badge/NixOS-Flake%20Ready-5277C3.svg?logo=nixos&logoColor=white)](flake.nix)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-DEA584.svg?logo=rust&logoColor=white)](Cargo.toml)

A complete, high-performance, Rust-native Linux userspace driver, RGB lighting controller, background daemon, and real-time audio-reactive music visualizer for the **Ajazz AK820 Max** mechanical keyboard.

Built specifically for **Linux & NixOS**, this project serves as a full, drop-in replacement for the vendor's Windows-only utility (`ajaz_ak820max.exe`).

---

## Features

- **Hardware Reverse-Engineered Support**:
  - **BY-COMBO / SinoWealth Revision** (`258A:010C` Wired & `3554:FA09` 2.4GHz Wireless): Uses 20-byte framed reports, modulo-256 checksums, and HID feature reports.
  - **Shinetek / EVision Revision** (`1A2C:A036` Wired & `1A2C:8FFF` 2.4GHz Wireless): Uses 64-byte `AA 55 CC 33` framed reports.
  - Auto-detection and seamless multi-interface protocol switching.
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
  - `ak820d`: Background user daemon managing auto-reconnect, hotplugging, and background audio visualization.
  - `ak820ctl`: Fast CLI utility with colorful terminal output, status queries, and interactive music mode.
- **First-Class NixOS Support**:
  - Self-contained Nix flake with `packages`, `apps`, `nixosModules`, and `devShells`.
  - Non-root `udev` access rules (`nix/70-ajazz-ak820.rules`).
  - Systemd user service (`nix/ak820d.service`).

---

## Supported Hardware

| Revision | Mode | USB VID:PID | Device String | Protocol Variant |
|---|---|---|---|---|
| **BY-COMBO** | Wired USB | `258A:010C` | "BY Tech Gaming Keyboard" | `ByComboWired` (Feature Reports) |
| **BY-COMBO** | 2.4GHz Wireless | `3554:FA09` | "Compx 2.4G Wireless Receiver" | `ByComboWireless` (20-byte framing) |
| **Shinetek** | Wired USB | `1A2C:A036` | "Shinetek Technology AK820" | `ShinetekEvision` (64-byte framing) |
| **Shinetek** | 2.4GHz Wireless | `1A2C:8FFF` | "2.4 Wireless Device" | `ShinetekEvision` (64-byte framing) |

---

## Project Structure

```
ajazz-ak820-max/
├── Cargo.toml          # Workspace root
├── flake.nix           # NixOS Flake definition
├── flake.lock          # Pinned flake inputs
├── hid/                # USB/HID transport, protocol framing & auto-discovery
├── rgb/                # Physical 82-key layout, 108 LED map & color math
├── audio/              # CPAL audio capture, FFT spectrum analyzer & music renderer
├── daemon/             # Background daemon (ak820d) with Unix socket IPC
├── cli/                # Command-line utility (ak820ctl)
├── nix/                # Nix package derivation, NixOS module, udev rules & service
├── docs/               # Protocol specification, architecture & user guides
└── tests/              # Protocol, FFT, and layout integration test suite
```

---

## Installation & Usage

### 1. Nix Flakes (NixOS)

Run commands directly without prior installation:

```bash
# Query keyboard status & battery
nix run .#ak820ctl -- status

# List detected devices
nix run .#ak820ctl -- devices

# Start real-time music sync visualization
nix run .#ak820ctl -- music --style spectrum
```

### 2. Cargo Build

```bash
# Enter development shell (provides cargo, rustc, alsa-lib, udev, pkg-config)
nix develop
# or: nix-shell -p cargo rustc pkg-config alsa-lib udev

# Compile release binaries
cargo build --release

# Run CLI
./target/release/ak820ctl status
```

### 3. Run Test Suite

```bash
cargo test --all
```

---

## CLI Reference (`ak820ctl`)

```bash
# Display battery percentage, charging state, connection mode
ak820ctl status

# List all detected Ajazz keyboards
ak820ctl devices

# Set built-in hardware effect
ak820ctl effect rainbow
ak820ctl effect breathing --color cyan --speed 4 --brightness 5
ak820ctl effect wave --direction left

# Set solid static color
ak820ctl static "#00FF88"

# Real-time music synchronization
ak820ctl music --style spectrum              # Rising equalizer columns
ak820ctl music --style pulse                 # Bass-reactive pulse
ak820ctl music --style wave                  # Expanding sound ripples
ak820ctl music --style passion               # High-energy rock flashes
ak820ctl music --style spectrum --daemon     # Stream in background via daemon

# Per-key custom coloring
ak820ctl key wasd red
ak820ctl key arrows blue
ak820ctl key esc "#FF0055"

# Configure hardware auto-sleep timeout
ak820ctl sleep 15    # 15 minutes
ak820ctl sleep 0     # Disable auto-sleep
```

---

## NixOS Declarative Configuration

### Using Flakes
Add this flake as an input to your system flake:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    ajazz-ak820-max.url = "github:AayamPokhrel/AJAZZ_AK820MAX_LINUX_DRIVER";
  };

  outputs = { self, nixpkgs, ajazz-ak820-max, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ajazz-ak820-max.nixosModules.default
        {
          hardware.ajazz-ak820-max.enable = true;
          hardware.ajazz-ak820-max.daemon.enable = true; # Starts ak820d in user session
        }
      ];
    };
  };
}
```

### Traditional `configuration.nix`
```nix
{ config, pkgs, ... }:

{
  imports = [
    /path/to/ajazz-ak820-max/nix/module.nix
  ];

  hardware.ajazz-ak820-max.enable = true;
  hardware.ajazz-ak820-max.daemon.enable = true;
}
```

---

## Sources, References & Acknowledgements

This driver was engineered through protocol reverse-engineering and interoperability analysis of:

- **[morluto/rea](https://github.com/morluto/rea)** — Reverse Engineer Anything toolchain reference.
- **[prasoondev/ajazz-linux](https://github.com/prasoondev/ajazz-linux)** — Protocol insights and Shinetek 64-byte framing documentation.
- **[gohv/EPOMAKER-Ajazz-AK820-Pro](https://github.com/gohv/EPOMAKER-Ajazz-AK820-Pro)** — Linux control reference for Ajazz keyboard variants.
- **Official Ajazz AK820 Max Windows Software** (`ajaz_ak820max.exe` / `OemDrv.exe` / `KB.ini` / `audiobar.txt`) — Reverse-engineered for BY-COMBO / SinoWealth 20-byte and 520-byte feature report structures.

---

## License & Fair Use

This project is licensed under either of:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

*Disclaimer*: This is an independent, open-source project developed solely for hardware interoperability on Linux systems. All trademarks and brand names are the property of their respective owners.
