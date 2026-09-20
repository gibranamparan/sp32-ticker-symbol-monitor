#!/usr/bin/env bash
# Rebuild the firmware and refresh the Wokwi flash layout.
#
# Usage: scripts/prepare-wokwi.sh [profile]   (default: release)
#
# Why: `cargo build` links the final Rust ELF *after* ESP-IDF's cmake step, so
# the app .bin inside the esp-idf-sys build dir goes stale. Wokwi loads the app
# via flasher_args.json -> libespidf.bin, so we regenerate that file from our
# current ELF with esptool before each simulation run.
set -euo pipefail
cd "$(dirname "$0")/.."

# ESP-IDF builds need the esp toolchain env, and a python with a working
# venv (the system python may lack ensurepip), for the IDF python env.
[ -f "$HOME/export-esp.sh" ] && . "$HOME/export-esp.sh"
[ -d "$HOME/.local/python312/bin" ] && export PATH="$HOME/.local/python312/bin:$PATH"

PROFILE="${1:-release}"
TARGET_DIR="target/xtensa-esp32-espidf/$PROFILE"
ELF="$TARGET_DIR/sp32-demo1"
ESPTOOL="${ESPTOOL:-$HOME/.local/bin/esptool}"

if [ "$PROFILE" = "release" ]; then
  cargo build --release
else
  cargo build
fi

BUILD_DIR="$(find "$TARGET_DIR/build" -type f -name flasher_args.json -path '*esp-idf-sys*' | head -1 | xargs dirname)"
echo "IDF build dir: $BUILD_DIR"

"$ESPTOOL" --chip esp32 elf2image \
  --flash-mode dio --flash-freq 40m --flash-size 4MB \
  -o "$BUILD_DIR/libespidf.bin" "$ELF"
echo "Refreshed $BUILD_DIR/libespidf.bin from current ELF"

# Keep wokwi.toml's firmware path in sync with the actual build dir hash
sed -i "s#^firmware = .*#firmware = \"$BUILD_DIR/flasher_args.json\"#" wokwi.toml
echo "wokwi.toml firmware -> $BUILD_DIR/flasher_args.json"
