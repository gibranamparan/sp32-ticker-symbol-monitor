## Context

Greenfield repo. Hardware: ESP-WROOM-32E module (classic ESP32, Xtensa LX6 dual-core, WiFi) on a standard 38-pin DevKit, plus an ILI9341 240×320 SPI TFT. Development host currently has only stable Rust; no Xtensa toolchain, no simulator tooling installed. The full exploration and decisions were agreed with the user before this proposal; this document pins them down.

## Goals / Non-Goals

**Goals:**
- One firmware codebase; the dev loop runs it in Wokwi with real HTTPS before any flashing
- Keep the app as "normal Rust" as possible (std) to minimize embedded-specific friction
- Prove toolchain → simulator → network end-to-end early, before display code

**Non-Goals:**
- Mocks in the application path (mocks only in unit tests, e.g. response parsing fixtures)
- Desktop-host abstraction layer (embedded-graphics-simulator variant) — considered and rejected
- QEMU as the emulation path — rejected: no WiFi emulation, and this app's core loop is networking
- Historical prices, sparklines, 24h deltas, multiple currencies, configuration UI

## Decisions

1. **std stack (`esp-idf-svc`/`esp-idf-hal`) over `esp-hal` (no_std).** HTTPS + JSON is the hard part of this app; with esp-idf we get ESP-IDF's mbedTLS-backed `EspHttpClient` and full `serde_json`. Update (during apply): `reqwest` + `esp-mbedtls` was the original sketch, but reqwest has no TLS backend that runs on Xtensa (rustls/ring and OpenSSL both lack ESP32 support) and esp-idf-svc 0.53 exposes no esp-mbedtls feature — the built-in `EspHttpClient` (esp_http_client + mbedTLS, embedded CA via `server_certificate`) covers every spec requirement with less code, so it replaces reqwest. no_std (`reqwless` + `embedded-tls`) rejected as before.

2. **Xtensa toolchain via `espup`.** WROOM-32E is classic ESP32 (Xtensa), so the esp-rs Rust fork (`cargo +esp`) is required regardless of stack. `ldproxy` needed for `esp-idf-sys`; ESP-IDF itself is downloaded automatically by the build script. Sourced env via `export-esp.sh`.

3. **Wokwi as the emulation path, accessed through `wokwi-cli`.** The virtual ESP32 runs the real compiled ELF; `diagram.json` wires a virtual ILI9341 to the same pins as hardware; virtual WiFi tunnels raw TCP through the Wokwi IoT Gateway so the firmware performs its own real TLS. Requires a free Wokwi account token (`WOKWI_CLI_TOKEN`). Alternatives rejected: QEMU (no WiFi), Renode (no ILI9341 part, more setup), host-abstraction (user explicitly wants the real thing, not a second target).

4. **Coinbase spot API.** `GET https://api.coinbase.com/v2/prices/BTC-USD/spot` → `{"data":{"base":"BTC","currency":"USD","amount":"67432.15"}}`. Keyless, tiny JSON, stable schema. Alternatives rejected: CoinGecko, Binance (heavier payloads/rate-limit considerations).

5. **Display via `mipidsi` + `embedded-graphics`.** `mipidsi` is the maintained ILI9341 driver implementing `DrawTarget<Rgb565>`. Fonts from `embedded-graphics` (or `profont`) scaled up for the big price. Standard 38-pin DevKit wiring pinned in the spec: SCK=GPIO18, MISO=GPIO19, MOSI=GPIO23, CS=GPIO5, DC=GPIO2, RST=GPIO4, BLK=GPIO21, VSPI.

6. **TLS trust: embed the relevant root CA** (the self-signed GTS Root R4, trust anchor for `api.coinbase.com`'s Google Trust Services chain) into the firmware image rather than a full webpki root bundle — saves flash/RAM on a 4 MB part. The PEM file keeps a trailing NUL byte for `X509::pem_until_nul`. If Coinbase rotates CAs, update the embedded cert.

7. **Task layout:** FreeRTOS task A = display/UI; task B = fetch loop (WiFi connect → fetch → signal UI → sleep 10 s), communicating via a shared value + state (e.g. `Arc<Mutex<AppState>>` or a channel). Error states are drawn by the UI task, so network hiccups never corrupt rendering.

8. **Config via env/`sdkconfig.defaults`:** WiFi SSID/password and any display tweaks come from environment variables at build time (`esp-idf` convention) so the same binary works for Wokwi (gateway WiFi) and home hardware by changing config, not code.

## Risks / Trade-offs

- [Wokwi networking needs the private IoT Gateway (account token)] → set up `WOKWI_CLI_TOKEN` in Phase 2; the blink milestone verifies gateway connectivity before any display work
- [Xtensa cold builds are slow (~1 min+), ESP-IDF download is large] → acceptable one-time cost; warm incremental builds are fine
- [TLS heap pressure on classic ESP32 (~520 KB RAM)] → response payload is ~100 bytes; keep reqwest buffers minimal; if tight, drop to raw `esp-mbedtls` HTTPS later without changing specs
- [Coinbase CA rotation breaks pinned root] → error state is visible on screen; update embedded cert is a one-line config change
- [Wokwi sim time ≠ real time] → 10 s refresh cadence is unaffected in practice; verify interval on real hardware during flashing phase
- [`mipidsi` display variants (inversion/BGR) differ between ILI9341 modules] → init options kept in one config struct; adjust against the virtual display in Wokwi first, confirm on hardware

## Migration Plan

Not applicable (greenfield). Rollback = delete the crate; nothing existing depends on it.

## Open Questions

None. Wiring is assumed per the standard 38-pin DevKit (user-confirmed assumption, recorded in the specs).
