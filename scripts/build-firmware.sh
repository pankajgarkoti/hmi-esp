#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

if [ ! -f .env.local ]; then
  echo "missing .env.local; copy .env.local.example and fill Wi-Fi credentials" >&2
  exit 1
fi

export ESP_IDF_SYS_ROOT_CRATE=hmi-firmware
export ESP_IDF_SDKCONFIG_DEFAULTS="$repo_root/crates/hmi-firmware/sdkconfig.defaults"
# Native component changes are not tracked by Cargo. Rebuild their owner rather
# than silently reusing stale C objects.
cargo +esp clean -p esp-idf-sys --release --target xtensa-esp32s3-espidf
exec cargo +esp build \
  -p hmi-firmware \
  --release \
  --target xtensa-esp32s3-espidf \
  -Zbuild-std=std,panic_abort
