# Ajazz AK820 Max USB/HID Protocol Specification

Reconstructed through static reverse engineering of `/home/aayam422h/AJAZ/AJAZZ_AK820_MAX_driver_V1.0/ajaz_ak820max.exe` (`OemDrv.exe` v2.00, `KB.ini`, `audiobar.txt`) and cross-referencing Linux reference implementations (`morluto/rea`, `prasoondev/ajazz-linux`, `gohv/EPOMAKER-Ajazz-AK820-Pro`). Validated against active hardware connected over Linux USB and 2.4GHz wireless.

---

## 1. Hardware Variants & Device Identification

The Ajazz AK820 Max series ships with two distinct USB/HID controller architectures across different manufacturing batches:

| Revision | Mode | USB VID:PID | Device String | Configuration Interface |
|---|---|---|---|---|
| **BY-COMBO (Current)** | Wired USB | `258A:010C` | "BY Tech Gaming Keyboard" | Interface 1 (`UsagePage 0xFF00, Usage 0x0001`), Feature Reports |
| **BY-COMBO (Current)** | 2.4GHz Wireless | `3554:FA09` | "Compx 2.4G Wireless Receiver" | Interface 1 (`UsagePage 0xFF02, Usage 0x0002`), Report ID `0x13` & Feature `0x06` |
| **Shinetek / EVision** | Wired USB | `1A2C:A036` | "Shinetek Technology AK820" | Interface 0 (`UsagePage 0xFF02, Usage 0x0002`), 64-byte framed reports |
| **Shinetek / EVision** | 2.4GHz Wireless | `1A2C:8FFF` | "2.4 Wireless Device" | Interface 0 (`UsagePage 0xFF02, Usage 0x0002`), 64-byte framed reports |

---

## 2. Protocol Specification: Revision BY-COMBO (`258A:010C` & `3554:FA09`)

### 2.1 2.4GHz Wireless Mode (`3554:FA09`)

#### Framing
Communication over the configuration interface (Report ID `0x13`) uses fixed 20-byte packets:
```
Offset  0:  0x13  (Report ID)
Offset  1:  0x15  (Header magic / Command group)
Offset  2:  Command byte (e.g., 0x88 for live RGB stream, 0x8E for custom RGB, 0x07 for effect)
Offset  3:  Chunk sequence counter (0, 1, 2, ...)
Offset 4..17: 14 bytes payload data
Offset 18:  Sub-parameter / padding (0x00)
Offset 19:  Checksum: modulo-256 sum of bytes 0..18
```
Checksum formula:
$$\text{Checksum} = \left(\sum_{i=0}^{18} \text{Packet}[i]\right) \pmod{256}$$

#### Battery & Status Query
Queried via **HID Feature Report ID 0x06** (8 bytes total):
```
Host -> Device (Get Feature 0x06):
Offset 0: 0x06

Device -> Host Reply:
Offset 0: 0x06 (Report ID)
Offset 1: 0x00
Offset 2: Power State (0x01 = Charging, 0x02 = On Battery, 0x03 = Fully Charged)
Offset 3: Battery Level (0x64 = 100%, 0x00..0x64)
Offset 4..7: Internal flags and CRC (e.g. 64 64 C9 6B)
```

#### Live Music RGB Streaming
- Stream command: `0x88`
- Total matrix: 108 LEDs $\times$ 3 bytes (R, G, B) = 324 bytes.
- Transmitted as 24 sequential 20-byte packets carrying 14 bytes of RGB payload each.
- Streaming frame rates: 20–60 FPS.

---

### 2.2 Wired USB Mode (`258A:010C`)

Wired mode communicates exclusively via **HID Feature Reports** (`HIDIOCSFEATURE` / `HIDIOCGFEATURE`):

#### Effect Control (Feature Report 0x05 - 6 bytes)
```
Offset 0: 0x05 (Report ID)
Offset 1: Hardware Effect Mode (1..20)
Offset 2: Brightness (1..5)
Offset 3: Speed (1..5)
Offset 4: Direction (0 = Right, 1 = Left)
Offset 5: Palette Color Index (0 = Red, 1 = Green, ..., 7 = Rainbow)
```

#### Custom & Live RGB Matrix (Feature Report 0x06 - 520 bytes)
```
Offset 0:   0x06 (Report ID)
Offset 1..324: 108 LEDs × 3 bytes RGB (R, G, B at LED Index)
Offset 325..519: Zero padding / profile metadata
```

---

## 3. Protocol Specification: Revision Shinetek (`1A2C:A036` & `1A2C:8FFF`)

### Framing
All communication is framed in 64-byte HID input/output reports with magic prefix `AA 55 CC 33`:
```
Offset 0..3:  AA 55 CC 33
Offset 4:     Command Byte
Offset 5..63: Payload (zero-padded)
```

### Commands Table
| Command Byte | Function | Payload Structure |
|---|---|---|
| `0x07` | Set Lighting | `[5]=mode-1, [6]=brightness-1 (0..4), [7]=speed-1 (0..4), [8]=dir, [9]=color` |
| `0x08` | Factory Reset | No payload |
| `0x09` | Stream Begin | Sent before streaming live RGB frames |
| `0x0B` | Custom Mode Prepare | Sent before transmitting custom stored RGB |
| `0x0C` | Sleep Time | `[5]=index` (0 = 5m, 1 = 15m, 2 = 30m, 3 = never) |
| `0x0D` | Debounce Time | `[5]=index` (0, 1, 2) |
| `0x0E` | Battery & Status | Reply: `[5]=battery % (<=100), [6]=power state (1=charge, 2=bat, 3=full)` |
| `0xF0`–`0xF5` | Live RGB Chunks | 6 chunks $\times$ 54 bytes = 18 LEDs $\times$ 3 (R,G,B) = 108 LEDs total |
| `0xF6`–`0xFB` | Custom RGB Chunks | 6 chunks $\times$ 54 bytes = 18 LEDs $\times$ 3 (R,G,B) stored in EEPROM |

---

## 4. Physical Layout & Equalizer Mapping

### Audio Equalizer Bands (Derived from `audiobar.txt`)
The 82 physical keys of the AK820 Max are assigned to 10 distinct equalizer frequency bands across the board's columns:

| Band | Frequency Range | Physical Key Column (Bottom to Top) |
|---|---|---|
| **0** | Sub-Bass (40–100 Hz) | `LCtrl`, `LShift`, `CapsLock`, `Tab`, `Q`, `~`, `1`, `Esc`, `F1` |
| **1** | Bass (100–250 Hz) | `LWin`, `LAlt`, `Z`, `A`, `S`, `W`, `2`, `3`, `F2`, `F3` |
| **2** | Low-Mids (250–500 Hz) | `X`, `C`, `D`, `E`, `R`, `4`, `F4` |
| **3** | Mids (500–1000 Hz) | `V`, `F`, `G`, `T`, `5`, `6`, `F5` |
| **4** | Mid-Highs (1–2 kHz) | `Space`, `B`, `N`, `H`, `Y`, `U`, `7`, `F6`, `F7` |
| **5** | Presence (2–4 kHz) | `M`, `J`, `K`, `I`, `8`, `9`, `F8` |
| **6** | Brilliance (4–6 kHz) | `RAlt`, `<`, `>`, `L`, `O`, `P`, `0`, `F9` |
| **7** | Treble (6–8 kHz) | `Fn`, `?`, `;`, `'`, `[`, `-`, `=`, `F10`, `F11` |
| **8** | High Treble (8–12 kHz) | `RCtrl`, `Left`, `Down`, `RShift`, `Up`, `Enter`, `]`, `\`, `Backspace`, `Del`, `F12` |
| **9** | Air (12–16 kHz) | `Right`, `End`, `PgDn`, `PgUp`, `Home` |
