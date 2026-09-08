# ClkMonOff

Instant hardware-level screen, audio, and input killswitch via hotkey for Linux and Windows.

## Usage

```bash
clkmonoff --start <hotkey> --end <hotkey>
```

* **`--start`**: Hotkey to activate the killswitch.
* **`--end`**: Hotkey to deactivate and unlock.

### Hotkey Format
* Key names are **case-insensitive** (e.g. `Esc`, `esc`, `ESC` are all valid).
* Multiple keys must be delimited strictly by commas `,` (e.g. `Esc,Enter,0,w`).

### Cleanup
To purge all application data, cached kernel modules, and unload the driver:
```bash
clkmonoff clear
```

## Features / Killswitch Effects

1. **Physical Display Sleep:** Sends hardware DDC/CI power-off commands (`VCP 0xD6`) over I2C/DP-AUX to put monitors into standby mode.
2. **Instant Zero-Frame Blackout:** Applies an all-black Color Transformation Matrix (CTM) via kernel DRM on Linux, or spawns an instant topmost black overlay on Windows.
3. **Media Pausing:** Pauses active media playback via MPRIS (Linux D-Bus) or SMTC (Windows 10+).
4. **Hard Audio Kill:** Mutes system volume, microphones, zeroes ALSA DMA buffers, and drops Bluetooth audio packets (SCO/ISO).
5. **Input Lock:** Blocks all keyboard and mouse events until the `--end` hotkey sequence is entered.