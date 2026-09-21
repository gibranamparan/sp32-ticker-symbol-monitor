## Why

The project needs its first application: a Rust firmware for the ESP-WROOM-32E that shows the live Bitcoin price on a Waveshare/Good Display 2.7" e-paper panel. Iterating directly on hardware is slow, so the development loop must run in a simulator (Wokwi) where the real firmware binary and real HTTPS traffic to the price API are exercised — not mocks — before flashing to the physical board. (The original plan targeted an ILI9341 TFT; the display pivoted to the e-paper panel after the TFT module never produced a frame during hardware bring-up — see design.md.)

## What Changes

- New Rust firmware crate targeting the ESP32 (classic, Xtensa) using the std stack (`esp-idf-svc`/`esp-idf-hal` on ESP-IDF/FreeRTOS).
- Joins a WiFi network in station mode and fetches the BTC-USD spot price from the Coinbase public API over HTTPS (no API key).
- Renders the price as large text on the GDEY027T91 2.7" monochrome e-paper panel (264×176 landscape) over SPI on the driver board's fixed e-paper pins (`epd-waveshare` + `embedded-graphics`), refreshing only when the displayed content changes.
- Refreshes the price every 60 seconds; on network/API failure, draws an error state on screen instead of the price and retries.
- Wokwi simulation setup (`wokwi.toml`, `diagram.json`) so the same binary that runs on hardware boots, connects, fetches, and updates state in the simulator with real network access through the Wokwi IoT Gateway (display rendering is verified on hardware; the sim has no e-paper part).
- Toolchain bootstrap: Xtensa Rust fork via `espup`, `ldproxy`.

## Capabilities

### New Capabilities
- `price-fetching`: Connecting to WiFi and retrieving the BTC-USD spot price from Coinbase over HTTPS, with parsing, error handling, and the 60-second refresh cadence.
- `price-display`: Rendering the current price (and error states) on the GDEY027T91 e-paper panel, including the visual update behavior after each refresh.
- `wokwi-simulation`: Running the identical firmware binary in the Wokwi simulator for build/network/state verification with real end-to-end HTTPS through the Wokwi IoT Gateway, before any hardware flashing.

### Modified Capabilities

(none — greenfield project, no existing specs)

## Impact

- New crate scaffolded from `esp-idf-template` (std) at the repo root; all application code is new.
- New dev dependencies of the workflow: `espup`, `ldproxy`, Wokwi CLI (external tools, not code).
- Rust dependencies: `esp-idf-svc`, `esp-idf-hal`, `epd-waveshare` (git master, `epd2in7_v2` module — the published release predates the plain-2.7 driver), `embedded-graphics`, `embedded-hal`, `serde`/`serde_json`. HTTPS via ESP-IDF's built-in `EspHttpClient` (mbedTLS) with an embedded root CA — reqwest was dropped during apply: no TLS backend for reqwest runs on Xtensa.
- External services: `api.coinbase.com` spot price endpoint; Wokwi IoT Gateway (account token required).
- Hardware: Waveshare E-Paper ESP32 Driver Board Rev 3 + Good Display GDEY027T91 2.7" e-paper panel on the flex connector (no external display wiring; pins are the board's fixed e-paper pins).
