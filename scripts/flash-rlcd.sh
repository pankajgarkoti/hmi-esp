#!/bin/sh
# Explicit hardware mutation; run only with the RLCD-4.2 connected.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
port=${1:?Usage: scripts/flash-rlcd.sh /dev/cu.usbmodemXXXX}
release=target/xtensa-esp32s3-espidf/release
test -f "$release/hmi-firmware"
uv run --with esptool python -m esptool --chip esp32s3 elf2image \
  --flash-size 16MB --output "$release/hmi-firmware.bin" "$release/hmi-firmware"
uv run --with esptool python -m esptool --port "$port" --chip esp32s3 \
  --before usb-reset write-flash \
  0x0 "$release/bootloader.bin" \
  0x8000 "$release/partition-table.bin" \
  0x10000 "$release/hmi-firmware.bin"
