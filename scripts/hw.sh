#!/usr/bin/env bash
# Hardware path: build with production WiFi credentials, flash, and monitor.
#
# Usage: scripts/hw.sh [extra cargo args...]
#
# Credentials come from `toml.production` (see toml.production.example).
# Create it once; it is gitignored. Simulation builds never read it.
set -euo pipefail
cd "$(dirname "$0")/.."

# ESP-IDF builds need the esp toolchain env + a python with a working venv
[ -f "$HOME/export-esp.sh" ] && . "$HOME/export-esp.sh"
[ -d "$HOME/.local/python312/bin" ] && export PATH="$HOME/.local/python312/bin:$PATH"

if [ ! -f toml.production ]; then
  echo "error: toml.production not found." >&2
  echo "Create it from the template:" >&2
  echo "  cp toml.production.example toml.production   # then edit values" >&2
  exit 1
fi

# Build + flash + monitor. `runner = "espflash flash --monitor"` in
# .cargo/config.toml makes `cargo run` do all three; --config gives the
# fragment higher precedence than the Wokwi credentials in .cargo/config.toml.
exec cargo run --release --config toml.production "$@"
