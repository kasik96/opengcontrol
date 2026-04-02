# opengcontrol

Open-source CLI alternative to Logitech G software on macOS.

<img width="978" height="714" alt="image" src="https://github.com/user-attachments/assets/0a2f24ef-da88-4bbd-965f-f7d80054a4d5" />


Configure your Logitech G-series mouse — DPI, polling rate, onboard profiles — without installing Logitech G HUB or Logi Options+. Settings are written directly to the mouse's onboard flash memory, so they persist across USB reconnects and work on any computer.

> **Status:** Early development. Contributions welcome.

---

## Supported devices

| Device | Wired | Wireless (Unifying) |
|--------|-------|---------------------|
| G403 Prodigy | ✓ `046D:C083` | ✓ `046D:C082` |
| G403 HERO | ✓ `046D:C08F` | — |
| G305 LIGHTSPEED | — | ✓ `046D:C092` |
| G305 SE | — | ✓ `046D:C53F` |

Wireless mice connect through a Unifying receiver. opengcontrol automatically selects the correct HID++ interface on the receiver — no extra configuration needed.

More devices are easy to add — see [docs/ADDING_DEVICES.md](docs/ADDING_DEVICES.md).

---

## Requirements

- macOS 12 or later (Apple Silicon and Intel)
- Rust toolchain (`rustup` — see below)
- Mouse connected via USB

---

## Install

### From source

```bash
# 1. Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone the repo
git clone https://github.com/your-username/opengcontrol
cd opengcontrol

# 3. Build and install
cargo install --path crates/opengcontrol
```

After installation, `opengcontrol` will be on your PATH via `~/.cargo/bin`.

### Build without installing

```bash
cargo build --release
# Binary is at: target/release/opengcontrol
```

---

## macOS permission

macOS requires **Input Monitoring** permission to access HID devices.

The first time you run `opengcontrol`, you may see a permission error. Fix it with:

```
opengcontrol doctor --open-settings
```

This opens **System Settings → Privacy & Security → Input Monitoring** where you can add `opengcontrol` to the allowed list.

> **Note:** Close Logitech G HUB or Logi Options+ before using opengcontrol — they hold exclusive access to the device.

---

## Usage

### Interactive dashboard

```
opengcontrol tui
```

Opens a full-screen dashboard showing all device settings at once. Control it with the keyboard:

```
┌─ ◈ Logitech G403 HERO  046D:C08F ──────── live ─┐
│  ┌─ DPI ──────────────┐  ┌─ Polling Rate ───────┐ │
│  │  Current  1600 dpi │  │  Current  1000 Hz    │ │
│  │                    │  │                      │ │
│  │  800 [1600] 3200   │  │  250 500 [1000]      │ │
│  └────────────────────┘  └──────────────────────┘ │
│  ┌─ Profiles ────────────────────────────────────┐ │
│  │  Active  1 / 3                                │ │
│  │  Slots  400 · [1600] · 3200 · 6400            │ │
│  └───────────────────────────────────────────────┘ │
│  ┌─ Buttons ─────────────────────────────────────┐ │
│  │  [1]Left  [2]Right  [3]Middle  [4]Back        │ │
│  └───────────────────────────────────────────────┘ │
│  Tab/↑↓ Focus  ←→ Change  ↵ Apply  r Refresh  q Quit │
└────────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| `Tab` / `↓` | Focus next panel |
| `Shift+Tab` / `↑` | Focus previous panel |
| `←` / `→` | Change value (DPI or Polling panels) |
| `Enter` | Apply pending value to device |
| `r` | Refresh state from device |
| `q` / `Esc` | Quit |

The focused panel has a cyan border. An amber `[value]` means an unapplied change — press Enter to write it to the mouse.

---

### Check everything is working

```
opengcontrol doctor
```

```
  ◈  opengcontrol  doctor — system diagnostics
  ──────────────────────────────────────────────────

  ✓  HID API initialized
  ✓  HID devices visible                        15 total
  ✓  Supported Logitech device found            1 device(s)
     · Logitech G403 HERO  (046D:C08F)
  ✓  Device opened successfully
  ✓  No conflicting software running

  ──────────────────────────────────────────────────
```

### List connected devices

```
opengcontrol list
```

### Show current settings

```
opengcontrol info
```

```
  ◈  Logitech G403 HERO  046D:C08F
  ──────────────────────────────────────────────────
  DPI               800
  Polling rate      1000 Hz
  Active profile    1
```

---

### DPI

```bash
opengcontrol dpi get          # show current DPI
opengcontrol dpi set 1600     # set DPI (200–12000, multiples of 50)
opengcontrol dpi list         # show all supported DPI values
```

### Polling rate

```bash
opengcontrol poll get         # show current rate
opengcontrol poll set 1000    # set rate: 125 | 250 | 500 | 1000 Hz
opengcontrol poll list        # show supported rates
```

### Onboard profiles

The mouse stores up to 3 independent profiles in flash memory. Switching profiles changes all settings (DPI, polling rate, button assignments) at once.

```bash
opengcontrol profile list               # list all profiles
opengcontrol profile active             # show active profile index
opengcontrol profile switch 1           # activate profile 1

opengcontrol profile export 0 backup.toml   # save profile to file
opengcontrol profile import backup.toml     # write profile to mouse
```

Exported profiles are plain TOML files you can edit and share:

```toml
index = 0
dpi_slots = [400, 800, 1600, 3200, 6400]
active_dpi_slot = 2
polling_rate_hz = 1000
```

### JSON output (for scripting)

Every command supports `--output json`:

```bash
opengcontrol --output json dpi get
# {"dpi":1600,"sensor":0}

opengcontrol --output json info
# {"device":"Logitech G403 HERO","vid":"046D","pid":"C08F","dpi":1600,...}
```

### Multiple devices

If you have more than one supported mouse connected, target a specific one with `--device`:

```bash
opengcontrol list                          # find the path
opengcontrol --device /dev/... dpi set 800
```

---

## How it works

opengcontrol uses the **HID++ 2.0** protocol — the same protocol Logitech's own software uses — reverse-engineered from open-source projects and community documentation.

Communication goes through macOS's native IOKit HID layer (via the [`hidapi`](https://github.com/libusb/hidapi) library). No kernel extensions, no background daemon, no installer.

Each invocation opens the device, performs the requested operation, and exits. Settings written to onboard flash persist without opengcontrol running.

See [docs/PROTOCOL.md](docs/PROTOCOL.md) for the wire-format details.

---

## Project structure

```
opengcontrol/
├── crates/
│   ├── hidpp-core/          # HID++ 2.0 protocol (device-agnostic library)
│   ├── logitech-devices/    # Device registry: VID/PID and capability definitions
│   └── opengcontrol/        # CLI binary
└── docs/
    ├── PROTOCOL.md          # HID++ 2.0 wire format reference
    └── ADDING_DEVICES.md    # How to add support for new mice
```

`hidpp-core` and `logitech-devices` are independent crates — if you want to build a GUI or a different tool, you can use them as libraries.

---

## Adding a new device

1. Find the USB IDs (`opengcontrol doctor` or `system_profiler SPUSBDataType`)
2. Create `crates/logitech-devices/src/devices/<model>.rs` with a `DeviceInfo` const
3. Register it in `crates/logitech-devices/src/registry.rs`

See [docs/ADDING_DEVICES.md](docs/ADDING_DEVICES.md) for a full walkthrough.

---

## Contributing

Pull requests are welcome. Some good first areas:

- **New devices** — G502, G Pro, MX Master 3
- **RGB lighting** — feature `0x8071`, two zones on G403
- **Button remapping** — feature `0x1B04`
- **Linux support** — transport layer already abstracted

For protocol questions, the best references are [libratbag](https://github.com/libratbag/libratbag), [logiops](https://github.com/PixlOne/logiops), and the [HID++ 2.0 draft spec](https://lekensteyn.nl/files/logitech/logitech_hidpp_2.0_specification_draft_2012-06-04.pdf).

---

## License

Apache License 2.0

---

*Not affiliated with or endorsed by Logitech.*
