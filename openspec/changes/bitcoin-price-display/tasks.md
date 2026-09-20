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

## 4. Display driver

- [x] 4.1 Add the virtual ILI9341 to `diagram.json` wired to VSPI pins (SCK=18, MISO=19, MOSI=23, CS=5, DC=2, RST=4, BLK=21)
- [x] 4.2 Initialize the ILI9341 via `mipidsi` and clear the screen; verify pixels appear on the Wokwi virtual display
- [x] 4.3 Draw text with `embedded-graphics`; verify legibility and orientation (rotation) on the virtual display

## 5. Price fetching

- [x] 5.1 Implement WiFi station connect using build-time SSID/password config, with retry on failure
- [ ] 5.2 Embed the root CA for `api.coinbase.com` and fetch `GET /v2/prices/BTC-USD/spot` over HTTPS via `EspHttpClient` (ESP-IDF mbedTLS; reqwest has no Xtensa TLS backend — see design.md decision 1)
- [x] 5.3 Parse the response with `serde_json` into a price value; format with thousands separator
- [x] 5.4 Unit test the parser against a recorded real response fixture (and malformed-body cases); test on host (`cargo test`)

## 6. Application loop

- [x] 6.1 Define shared app state (current price / error state) between fetch and UI tasks
- [ ] 6.2 Implement the fetch task: connect → fetch → update state → sleep 10 s → repeat
- [ ] 6.3 Implement the UI task: waiting state at startup, big price on success, error state on failure with no known price, atomic full-frame redraws
- [ ] 6.4 End-to-end run in Wokwi: price from Coinbase appears on the virtual display and updates

## 7. Hardware bring-up

- [ ] 7.1 Wire the physical ILI9341 to the 38-pin DevKit per the pinned pin map
- [ ] 7.2 Flash the same binary; verify display rendering matches Wokwi (adjust inversion/rotation config only if the module differs)
- [ ] 7.3 Verify WiFi credentials config change and the 10 s refresh cadence on real hardware
