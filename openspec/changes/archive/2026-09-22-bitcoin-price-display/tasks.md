## 1. Toolchain bootstrap

- [x] 1.1 Install `espup` and `ldproxy` (`cargo install espup ldproxy`), run `espup install` to get the Xtensa Rust fork (`cargo +esp`)
- [x] 1.2 Verify the toolchain: a trivial `cargo +esp check` succeeds after sourcing `export-esp.sh`
- [x] 1.3 Document the toolchain setup steps in a top-level README (source export script, required env vars)

## 2. Project scaffold

- [x] 2.1 Generate the crate from `esp-idf-template` (ESP32, std) in the repo root
- [x] 2.2 Add dependencies: `esp-idf-hal`, `esp-idf-svc`, `mipidsi`, `embedded-graphics`, `embedded-hal`, `reqwest` (TLS feature with `esp-mbedtls`), `serde`, `serde_json`
- [x] 2.3 Confirm `cargo +esp build` produces a working ELF and log output over serial

## 3. Wokwi simulation setup

- [x] 3.1 Create `wokwi.toml` pointing at the built ELF and `diagram.json`
- [x] 3.2 Add `wokwi-esp32-devkit-v1` and a blink/serial hello-world, run via `wokwi-cli` with `WOKWI_CLI_TOKEN`
- [x] 3.3 Verify WiFi networking through the Wokwi IoT Gateway (e.g. DNS/TCP smoke test from the virtual ESP32)
- [x] 3.4 Add a single simulation entry-point command (documented in README)

## 4. Display driver (ILI9341 — superseded by §8, kept as history)

- [x] 4.1 Add the virtual ILI9341 to `diagram.json` wired to VSPI pins (SCK=18, MISO=19, MOSI=23, CS=5, DC=2, RST=4, BLK=21)
- [x] 4.2 Initialize the ILI9341 via `mipidsi` and clear the screen; verify pixels appear on the Wokwi virtual display
- [x] 4.3 Draw text with `embedded-graphics`; verify legibility and orientation (rotation) on the virtual display

## 5. Price fetching

- [x] 5.1 Implement WiFi station connect using build-time SSID/password config, with retry on failure
- [x] 5.2 Embed the root CA for `api.coinbase.com` and fetch `GET /v2/prices/BTC-USD/spot` over HTTPS via `EspHttpClient` (ESP-IDF mbedTLS; reqwest has no Xtensa TLS backend — see design.md decision 1)
- [x] 5.3 Parse the response with `serde_json` into a price value; format with thousands separator
- [x] 5.4 Unit test the parser against a recorded real response fixture (and malformed-body cases); test on host (`cargo test`)

## 6. Application loop

- [x] 6.1 Define shared app state (current price / error state) between fetch and UI tasks
- [x] 6.2 Implement the fetch task: connect → fetch → update state → sleep 60 s → repeat
- [x] 6.3 Implement the UI task: waiting state at startup, big price on success, error state on failure with no known price, atomic full-frame redraws
- [x] 6.4 End-to-end run in Wokwi: price from Coinbase appears on the virtual display and updates

## 7. Hardware bring-up (e-paper)

- [x] 7.1 Attach the GDEY027T91 panel to the driver board's e-paper flex connector and set the pixel-config switch to the position matching the panel
- [x] 7.2 Flash the same binary; verify the waiting/price/error screens render on the physical e-paper (orientation, contrast, legibility)
- [x] 7.3 Verify WiFi credentials config change and the 60 s refresh cadence on real hardware

## 8. E-paper migration

- [x] 8.1 Swap `mipidsi` for `epd-waveshare` (git master, `epd2in7_v2`) in `Cargo.toml`
- [x] 8.2 Rework display bring-up: driver-board fixed pins (SCK=13, MOSI=14, CS=15, DC=27, RST=26, BUSY=25), landscape rotation, no backlight/reset hacks
- [x] 8.3 Rework UI rendering for monochrome (`Color`) + refresh only when displayed content changes
- [x] 8.4 Update `diagram.json` for Wokwi (remove ILI9341, hold BUSY idle) and verify boot + live fetch in simulation
- [x] 8.5 Repurpose `PANEL_TEST` as the e-paper bring-up pattern (single full update) and verify on hardware
