# Live Human Nursery M0 — Hardware Readiness Evidence

**Date:** 2026-05-24
**Machine:** MacBook Pro M1
**OS:** macOS 26 (Darwin 25.3.0)

## Build

- `cargo test`: PASS (all tests pass)
- `cargo build --release`: PASS (nokhwa + cpal + coreaudio all link)

## Hardware Code Status

| Channel | Code | Dependency | macOS API |
|---------|------|-----------|-----------|
| Camera (PHOT) | `RealCameraFrameProvider::try_new()` | nokhwa 0.10 | AVFoundation |
| Microphone (VIB) | `VibAdapter::new()` | cpal 0.15 | CoreAudio |
| Speaker (Vocal) | `spawn_vocal_output()` | cpal 0.15 | CoreAudio |

All three hardware paths are fully implemented. `body.rs` tries real camera first, falls back to `StubRealCameraFrameProvider` on failure.

## Sandbox Smoke Test

In Claude sandbox (no camera/mic permission):
- body process: runs, writes `snapshot.bin` (300KB) and `learned_bias.bin`
- observer: reads snapshots, outputs numeric TSV
- health/phot_h: FAULT (expected — sandbox blocks AVFoundation)
- snap_fails: 0

## Notes

- Camera permission: first real-terminal run will trigger macOS permission dialog. If previously denied, enable in System Settings → Privacy & Security → Camera.
- Microphone permission: same as above for Microphone.
- The `nice(5) failed` warning on macOS is harmless (sandbox restriction on priority change).
- Startup script uses `$TMPDIR` by default; for real nursery sessions, use a persistent directory.
