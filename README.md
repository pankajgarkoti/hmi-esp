# hmi-esp

## Living Display — RLCD fork

This fork turns the **Waveshare ESP32-S3-RLCD-4.2** into a landscape living display:
16 selectable cellular automata, microphone-driven perturbations, SD-backed worlds,
and a brief network-synchronized clock glance. It uses small local acoustic
fingerprints for the sound of **"time"** and a transient detector for **finger snaps**;
there is no speech-to-text engine, neural speech model, or cloud audio service.

- **KEY click:** clock for five seconds (simulation continues); click again to return.
- **BOOT click:** Settings / next row. **BOOT hold:** change selected value.
- **KEY hold:** return to the living field. **PWR hold:** hardware power off.
- Default: Conway, **4 generations/second**, subtle microphone perturbations.
- Set **MICROPHONE → OFF** for exact, unperturbed rules; clock sound cues stay enabled.
- **AUTO TOUR:** Off / 30 seconds / 2 minutes / 5 minutes per world.
- **NEW SEED:** deliberately replace only the current world's grid.
- **TEACH TIME:** hold, release, then say "time" once. Stores only a small acoustic
  fingerprint. This is the best option for a different voice or room; similar sounds
  can match, and the detector is not general speech recognition.
- **WI-FI SETUP:** select it in Settings and hold BOOT. Join the temporary
  `Living-RLCD` network on your phone using the password shown on the display,
  accept "stay connected without internet" if prompted, then open the address
  shown there and enter the destination **2.4 GHz** network.
  This setup window closes after three minutes or when you press KEY. New
  credentials are stored on the board only after the connection succeeds.
- Battery percentage appears on both the living field and the clock; it is an
  approximate voltage-derived reading, not a fuel-gauge measurement.

Worlds resume when selected. With an SD card, snapshots are saved on switching and
every 30 seconds (unchanged worlds are skipped). Abrupt power-off can lose the
latest interval. Files live under `/sdcard/living/`, with checksums and a previous
copy for recovery. Without SD, worlds survive switches but not a power cycle.

Build and flash (requires the `esp` Rust toolchain, `ldproxy`, CMake, Ninja, Python and `uv`):

```sh
# .env.local: WIFI_SSID=... and WIFI_PASSWORD=... (no surrounding quotes)
./scripts/vendor-sync.sh
. ~/export-esp.sh
./scripts/build-firmware.sh
sh scripts/flash-rlcd.sh /dev/cu.usbmodem2101
```

The flash command is only for **RLCD-4.2**, not Touch349. Keep `.env.local`, generated
firmware images (which contain Wi-Fi credentials), and device backups private.

See [Living Display design, research and verification](docs/living-display.md).
The original project documentation and other board targets follow below.

---

Reference and integration workspace for a Rust-first HMI on the exact
Waveshare ESP32-S3-RLCD-4.2 and ESP32-S3-Touch-LCD-3.49 V2 platforms.

The intended product is not a fork of any single example in `vendor/`. It is a
local-rendering device with a shared Rust UI core, a deterministic host
simulator, and an ESP-IDF firmware shell for Wi-Fi, storage, audio, sensors, and
USB Serial/JTAG. HTTP/WebSocket transport, a versioned remote protocol, and OTA
remain planned rather than implemented.

## Touch LCD 3.49 V2 UI emulator

The full browser emulator runs without hardware. It models touch flows,
recording, files, playback, diagnostics, calibration, settings, power, and
deterministic fault scenarios.

```sh
./scripts/open-touch349-emulator.sh
```

These screenshots come from the current emulator at the exact 172x640 native
viewport. They do not prove physical-device behavior.

| Home | Recorder | Files |
| --- | --- | --- |
| <img src="artifacts/touch349-emulator/home.png" alt="Touch349 emulator home screen" width="172"> | <img src="artifacts/touch349-emulator/recorder.png" alt="Touch349 emulator recorder screen" width="172"> | <img src="artifacts/touch349-emulator/files.png" alt="Touch349 emulator files screen" width="172"> |
| Diagnostics | Settings | SD fault |
| <img src="artifacts/touch349-emulator/diagnostics.png" alt="Touch349 emulator diagnostics screen" width="172"> | <img src="artifacts/touch349-emulator/settings.png" alt="Touch349 emulator settings screen" width="172"> | <img src="artifacts/touch349-emulator/sd-fault.png" alt="Touch349 emulator SD fault screen" width="172"> |

The emulator is a product-design tool. The current physical proof covers LCD
output, SD mount, and the active-low GPIO16 power path. Other device states in
the emulator are simulations until they pass direct hardware tests.

Start with:

- [`docs/build-intent.md`](docs/build-intent.md) for the product boundary and
  reference-by-reference analysis.
- [`vendor/README.md`](vendor/README.md) for pinned source management and the
  local Cargo patch layer.
- [`vendor/sources.lock`](vendor/sources.lock) for exact upstream revisions.
- [`docs/firmware.md`](docs/firmware.md) for the portrait device UI, hierarchical
  controls, opt-in recorder, SD viewer, audio player, and build commands.

The upstream checkouts are intentionally ignored by the top-level Git repo.
Restore or verify them with:

```sh
./scripts/vendor-sync.sh
./scripts/vendor-check.sh
```

The first firmware and host simulator now live under `crates/`. Build and test
commands are documented in `docs/firmware.md`. No hardware is flashed by any
build or simulator command.
