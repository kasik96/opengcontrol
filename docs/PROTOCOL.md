# HID++ 2.0 Protocol Notes

This document summarizes the reverse-engineered HID++ 2.0 protocol used by Logitech G-series mice.

## References

- [HID++ 2.0 draft spec (2012)](https://lekensteyn.nl/files/logitech/logitech_hidpp_2.0_specification_draft_2012-06-04.pdf)
- [Logitech cpg-docs (official partial docs)](https://github.com/Logitech/cpg-docs/tree/master/hidpp20)
- [Solaar feature list](https://pwr-solaar.github.io/Solaar/features/)
- [logiops source (C++)](https://github.com/PixlOne/logiops)
- [cvuchener/hidpp (C++ tools)](https://github.com/cvuchener/hidpp)

## Message Format

HID++ 2.0 uses two message sizes:

### SHORT (7 bytes)
```
Byte 0: Report ID = 0x10
Byte 1: Device ID (0xFF for wired/directly-connected)
Byte 2: Feature Index (runtime, device-specific)
Byte 3: (Function << 4) | Software_ID
Bytes 4-6: Parameters
```

### LONG (20 bytes)
```
Byte 0: Report ID = 0x11
Byte 1: Device ID
Byte 2: Feature Index
Byte 3: (Function << 4) | Software_ID
Bytes 4-19: Parameters (16 bytes)
```

## Feature Discovery

Every device has a feature table. Codes (like `0x2201`) map to runtime indices via IRoot.

```
1. Send to feature index 0x00, function 0: GetFeature(feature_code_u16)
   → params: [code_hi, code_lo, 0, 0]
2. Response params[0] = runtime_index (0 = not present)
3. Cache index; use it for all subsequent calls to that feature
```

## Key Feature Codes

| Code   | Name             | Notes                          |
|--------|------------------|--------------------------------|
| 0x0000 | IRoot            | Always at index 0              |
| 0x0001 | IFeatureSet      | Enumerate all features         |
| 0x2201 | Adjustable DPI   | Get/set sensor DPI             |
| 0x8060 | Report Rate      | Get/set polling rate           |
| 0x8070 | RGB Effects      | Single-zone color effects      |
| 0x8071 | Per-Key Lighting | Per-zone RGB (some devices)    |
| 0x8110 | Onboard Profiles | Flash memory profiles          |

## DPI (Feature 0x2201)

### Function 0 — getSensorCount
- Params: none
- Response params[0]: sensor count (G403 = 1)

### Function 1 — getSensorDpiList
- Params: [sensor_idx, 0...]
- Response: pairs of big-endian u16 DPI values, terminated by 0x0000
- Range format: if first value has high byte 0xE0: [min|0xE000, step, max]

### Function 2 — getSensorDpi
- Params: [sensor_idx, 0, 0, 0]
- Response params[1-2]: current DPI (big-endian u16)

### Function 3 — setSensorDpi
- Params: [sensor_idx, dpi_hi, dpi_lo, 0]
- Sets session DPI (not persisted to flash without profile write)

## Polling Rate (Feature 0x8060)

Rate index mapping:
- 0x01 = 1000 Hz
- 0x02 = 500 Hz
- 0x03 = 250 Hz
- 0x04 = 125 Hz

### Function 0 — getReportRateList
- Response params[0]: bitmask of supported rates (bit 0 = 125Hz, ..., bit 3 = 1000Hz)

### Function 1 — getReportRate
- Response params[0]: current rate index

### Function 2 — setReportRate
- Params: [rate_index, 0, 0, 0]

## Onboard Profiles (Feature 0x8110)

Profile data is stored in pages of 16 bytes each in the mouse's flash memory.
Each profile occupies multiple pages.

### Function 0 — getProfileDescriptors
- Response: [memory_model, profile_format, macro_format, profile_count, ...]

### Function 1 — setCurrentProfile
- Params: [profile_index, 0, 0, 0]

### Function 2 — getCurrentProfile
- Response params[0]: active profile index

### Function 3 — readFromFlash (LONG)
- Params: [page_addr, 0...]
- Response: 16 bytes of profile data

### Function 4 — writeToFlash (LONG)
- Params: [page_addr, data_byte_0..data_byte_15]

## macOS Notes

- Use IOKit backend (not libusb) for HID access
- hidapi uses IOHIDManager on macOS
- Input Monitoring permission required (System Settings → Privacy & Security)
- The `write()` call via hidapi on macOS requires a leading 0x00 byte (report ID prefix)
- Logitech G HUB must be closed before opengcontrol can open the device
