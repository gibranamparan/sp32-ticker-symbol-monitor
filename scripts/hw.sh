#!/usr/bin/env bash
# Hardware path: build with production WiFi credentials, then flash.
#
# Usage: scripts/hw.sh [--no-monitor] [extra cargo build args...]
#   (default)     build + flash + attach the serial monitor (Ctrl+C to exit)
#   --no-monitor  build + flash only, exit when done
#
# Credentials come from `toml.production` (see toml.production.example).
# Create it once; it is gitignored. Simulation builds never read it.
set -euo pipefail
cd "$(dirname "$0")/.."

# ESP-IDF builds need the esp toolchain env + a python with a working venv
[ -f "$HOME/export-esp.sh" ] && . "$HOME/export-esp.sh"
[ -d "$HOME/.local/python312/bin" ] && export PATH="$HOME/.local/python312/bin:$PATH"

MONITOR=1
if [ "${1:-}" = "--no-monitor" ]; then
  MONITOR=0
  shift
fi

if [ ! -f toml.production ]; then
  echo "error: toml.production not found." >&2
  echo "Create it from the template:" >&2
  echo "  cp toml.production.example toml.production   # then edit values" >&2
  exit 1
fi

# Build with the production env fragment (highest cargo config precedence)
cargo build --release --config toml.production "$@"
ELF=target/xtensa-esp32-espidf/release/sp32-demo1

if [ "$MONITOR" = 1 ]; then
  exec espflash flash --monitor "$ELF"
else
  exec espflash flash "$ELF"
fi
