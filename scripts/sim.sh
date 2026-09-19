#!/usr/bin/env bash
# One-command simulation: build -> refresh Wokwi flash image -> run in Wokwi.
# Requires WOKWI_CLI_TOKEN in the environment (see README).
# Extra args are passed to wokwi-cli, e.g.: scripts/sim.sh --timeout 60000
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -z "${WOKWI_CLI_TOKEN:-}" ]; then
  echo "error: WOKWI_CLI_TOKEN is not set (see README.md)" >&2
  exit 1
fi

./scripts/prepare-wokwi.sh release
exec wokwi-cli "$@"
