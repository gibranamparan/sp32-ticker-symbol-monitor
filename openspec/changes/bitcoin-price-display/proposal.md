## Why

The project needs its first application: a Rust firmware for the ESP-WROOM-32E that shows the live Bitcoin price on an ILI9341 TFT. Iterating directly on hardware is slow, so the development loop must run in a simulator (Wokwi) where the real firmware binary, the real display driver, and real HTTPS traffic to the price API are exercised — not mocks — before flashing to the physical board.

## What Changes

- New Rust firmware crate targeting the ESP32 (classic, Xtensa) using the std stack (`esp-idf-svc`/`esp-idf-hal` on ESP-IDF/FreeRTOS).
- Joins a WiFi network in station mode and fetches the BTC-USD spot price from the Coinbase public API over HTTPS (no API key).
- Renders the price as large text on a 240×320 ILI9341 display over SPI (`mipidsi` + `embedded-graphics`).
- Refreshes the price every 10 seconds; on network/API failure, draws an error state on screen instead of the price and retries.
- Wokwi simulation setup (`wokwi.toml`, `diagram.json`) with the virtual ESP32 + ILI9341, so the same binary that runs on hardware runs in the simulator with real network access through the Wokwi IoT Gateway.
- Toolchain bootstrap: Xtensa Rust fork via `espup`, `ldproxy`.

## Capabilities

### New Capabilities
- `price-fetching`: Connecting to WiFi and retrieving the BTC-USD spot price from Coinbase over HTTPS, with parsing, error handling, and the 60-second refresh cadence.
- `price-display`: Rendering the current price (and error states) on the ILI9341 display, including the visual update behavior after each refresh.
- `wokwi-simulation`: Running the identical firmware binary in the Wokwi simulator with a virtual ILI9341 and real end-to-end HTTPS through the Wokwi IoT Gateway, before any hardware flashing.

### Modified Capabilities

(none — greenfield project, no existing specs)

## Impact

- New crate scaffolded from `esp-idf-template` (std) at the repo root; all application code is new.
- New dev dependencies of the workflow: `espup`, `ldproxy`, Wokwi CLI (external tools, not code).
- Rust dependencies: `esp-idf-svc`, `esp-idf-hal`, `mipidsi`, `embedded-graphics`, `embedded-hal`, `serde`/`serde_json`. HTTPS via ESP-IDF's built-in `EspHttpClient` (mbedTLS) with an embedded root CA — reqwest was dropped during apply: no TLS backend for reqwest runs on Xtensa.
- External services: `api.coinbase.com` spot price endpoint; Wokwi IoT Gateway (free account token required).
- Hardware assumed: ESP-WROOM-32E on a standard 38-pin DevKit + ILI9341 module wired to VSPI (pins pinned in design.md).
