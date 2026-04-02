# CLAUDE.md — opengcontrol developer notes

## Build & test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

All four must pass before committing. CI runs macOS and Windows jobs.

## Workspace layout

```
crates/
  hidpp-core/        # HID++ 2.0 protocol — platform-agnostic, no OS calls
  logitech-devices/  # Static device registry (VID/PID, capabilities)
  opengcontrol/      # CLI binary — only crate with platform-specific code
```

`hidpp-core` and `logitech-devices` have zero platform-specific code. All `#[cfg(target_os)]` lives in `crates/opengcontrol/src/`.

## Platform support

Supported: **macOS** (IOKit via hidapi), **Windows** (Windows HID API via hidapi).

Platform-specific code lives in exactly two files:

| File | What varies |
|------|-------------|
| `crates/opengcontrol/src/permissions.rs` | Permission guidance text; `open_input_monitoring_settings()` (macOS only) |
| `crates/opengcontrol/src/cli/doctor.rs` | Process detection (`pgrep` vs `tasklist`); permission fix instructions; `--open-settings` flag (macOS only); install hint |

### Adding Linux support

The two stubs already compile on Linux (the `#[cfg(not(any(target_os = "macos", target_os = "windows")))]` blocks). To complete Linux support:

1. `permissions.rs` — the generic fallback prints a udev hint; extend if needed.
2. `doctor.rs` — `is_process_running` has no Linux branch. Add a `#[cfg(target_os = "linux")]` variant using `pgrep` (same as macOS/unix — the existing `#[cfg(unix)]` branch already covers Linux).
3. Test that `hidapi` enumerates devices correctly (requires udev rules or running as root).

### hidapi feature flag

`macos-shared-device` (IOKit shared-device mode) is enabled per-platform in each crate's `Cargo.toml`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
hidapi = { workspace = true, features = ["macos-shared-device"] }
```

Do not put this feature in `[workspace.dependencies]` — it must stay macOS-only.

## HID report ID convention

All `to_write_buf()` methods in `hidpp-core/src/protocol/message.rs` prepend a `0x00` byte. This is required by hidapi on **all platforms** (the report ID byte for devices that don't use numbered reports). Do not make this conditional.

## Commit style

- Author: Martin Kase — no `Co-Authored-By` trailers.
- Single focused commit per logical change.
