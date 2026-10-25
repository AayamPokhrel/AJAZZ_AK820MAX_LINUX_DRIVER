# Ajazz AK820 Max — Linux User Guide

This guide covers building, running, configuring, and packaging the native Linux driver and music visualizer for the **Ajazz AK820 Max** mechanical keyboard.

---

## 1. Quick Start

### 1.1 Running with Nix Flakes (NixOS)

To run without installing directly:

```bash
# Query keyboard status & battery
nix run .#ak820ctl -- status

# List detected devices
nix run .#ak820ctl -- devices

# Start real-time music sync visualization
nix run .#ak820ctl -- music --style spectrum
```

### 1.2 Building with Cargo

From the project root:

```bash
# Enter development shell with dependencies (ALSA, pkg-config, udev)
nix develop

# Build workspace in release mode
cargo build --release

# Run CLI
./target/release/ak820ctl status
```

---

## 2. CLI Usage (`ak820ctl`)

The `ak820ctl` utility provides complete control over lighting, audio sync, and keyboard settings:

### Device Status
```bash
ak820ctl status
```
Output:
```text
=== Ajazz AK820 Max Status ===
  Source:          Direct (Compx 2.4G Wireless Receiver)
  Battery:         100%
  Power State:     Fully Charged
```

### Hardware Lighting Effects
Select built-in animations:
```bash
# Set Rainbow Spectrum
ak820ctl effect rainbow

# Set Breathing effect in Cyan with custom speed and brightness
ak820ctl effect breathing --color cyan --speed 5 --brightness 4

# Set Sine Wave effect moving left
ak820ctl effect wave --direction left

# Other supported effects:
# static, breathing, wave, spectrum, starry, ripple, ejection,
# kaleidoscope, cross, reactive, scroll, top-bottom, rain, windmill, off
```

### Static Color
Set a single solid color:
```bash
# Color name
ak820ctl static purple

# Hex color
ak820ctl static "#00FFC8"
```

### Real-Time Music Reactive RGB
Stream music visualization from your system audio output:

```bash
# Spectrum Equalizer (Equalizer bars rising across the 10 keyboard columns)
ak820ctl music --style spectrum

# Bass Pulse (Full-board reactive pulse matching musical beat)
ak820ctl music --style pulse

# Radial Wave (Circular ripple expanding outwards on bass hits)
ak820ctl music --style wave

# Soft Dance (Smooth rhythmic gradient flow)
ak820ctl music --style soft-dance

# Passion (High-energy rock style with sharp transient flashes)
ak820ctl music --style passion

# Custom tint with higher sensitivity and 60 FPS:
ak820ctl music --style spectrum --color "#FF3366" --sensitivity 1.5 --fps 60

# Run in background via daemon:
ak820ctl music --style spectrum --daemon
```

### Per-Key Custom Colors
```bash
# Set WASD cluster to red
ak820ctl key wasd red

# Set arrow keys to green
ak820ctl key arrows green

# Set specific key
ak820ctl key esc "#FF0055"
```

### Sleep Timeout
Configure hardware auto-sleep timer:
```bash
ak820ctl sleep 15    # 15 minutes
ak820ctl sleep 0     # Never sleep
```

---

## 3. Background Daemon (`ak820d`)

The daemon runs continuously in the user session, handling:
- Device hotplugging and auto-reconnection (when toggling between USB cable and 2.4GHz receiver).
- Background audio capture and real-time music streaming.
- IPC commands from scripts, keybindings, or desktop widgets via Unix domain socket (`/run/user/$UID/ajazz-ak820.sock`).

### Starting the Daemon
```bash
# Run foreground (for testing/logs)
ak820d -v

# Stop music streaming via CLI
ak820ctl daemon stop-music
```

---

## 4. NixOS Integration

### 4.1 Using Flakes in `flake.nix`

Add the input to your system flake:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    ajazz-ak820-max.url = "path:/home/aayam422h/AJAZ/ajazz-ak820-max";
  };

  outputs = { self, nixpkgs, ajazz-ak820-max, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
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

### 4.2 Standard `configuration.nix`

If not using flakes, import the module directly:

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

## 5. Linux Audio Setup (PipeWire & PulseAudio)

The audio engine automatically detects default loopback monitors:
- **PipeWire**: Works out of the box. CPAL monitors your active audio sink.
- **PulseAudio**: Works out of the box via the ALSA Pulse plugin or loopback monitor.
- **ALSA**: Direct hardware capture supported.
