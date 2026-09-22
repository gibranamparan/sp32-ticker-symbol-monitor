## Context

Greenfield repo. Hardware: Waveshare E-Paper ESP32 Driver Board Rev 3 (ESP32-WROOM-32, classic Xtensa LX6, WiFi) with a Good Display GDEY027T91 2.7" monochrome e-paper panel (SSD1680, 264×176, SPI) attached through the board's native 24-pin flex connector. Development host currently has only stable Rust; no Xtensa toolchain, no simulator tooling installed. The full exploration and decisions were agreed with the user before this proposal; this document pins them down, including the later pivot from the originally planned ILI9341 TFT to the e-paper panel.

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

3. **Wokwi as the emulation path, accessed through `wokwi-cli`.** The virtual ESP32 runs the real compiled ELF; virtual WiFi tunnels raw TCP through the Wokwi IoT Gateway so the firmware performs its own real TLS. Update (during apply, e-paper pivot): Wokwi has no matching e-paper part, so the simulation now validates build/WiFi/fetch/state basics with the e-paper `BUSY` line held idle in `diagram.json`; virtual display rendering is dropped and the panel is verified on hardware. Requires a free Wokwi account token (`WOKWI_CLI_TOKEN`). Alternatives rejected as before: QEMU (no WiFi), Renode (more setup), host-abstraction (user wants the real firmware).

4. **Coinbase spot API.** `GET https://api.coinbase.com/v2/prices/BTC-USD/spot` → `{"data":{"base":"BTC","currency":"USD","amount":"67432.15"}}`. Keyless, tiny JSON, stable schema. Alternatives rejected: CoinGecko, Binance (heavier payloads/rate-limit considerations).

5. **Display via `epd-waveshare` on the GDEY027T91 e-paper panel.** The panel (SSD1680 controller, 264×176, black/white, full refresh 3 s / fast refresh 1.5 s / partial refresh 0.3 s per vendor) hangs off the driver board's fixed e-paper pins: SCK=GPIO13, MOSI=GPIO14, CS=GPIO15, DC=GPIO27, RST=GPIO26, BUSY=GPIO25 — no external wiring, no backlight. Driver: `epd-waveshare` (git master) using the `epd2in7_v2` module — the published 0.6.0 crate predates the plain-2.7 module; it implements `embedded-hal` 1.0 `SpiDevice`/`DelayNs` and embedded-graphics-core 0.4, matching esp-idf-hal 0.47 and embedded-graphics 0.8. Rendering uses the crate's `Display2in7` full-frame buffer + `Color` (B/W), rotated to the 264×176 landscape layout; the UI redraws (and refreshes) only when the displayed content changes. Update (during apply, display pivot): the originally planned ILI9341 TFT (`mipidsi`) was abandoned after the physical module never produced a frame despite verified SPI activity, power, and reset at its pins — the module is presumed faulty. The e-paper panel is the native pairing for this board and a better fit for a ticker (bistable image, no backlight, daylight readable).

6. **TLS trust: embed the relevant root CA** — currently the self-signed GTS Root R1 (pki.goog), trust anchor for the RSA chain (`coinbase.com` ← GTS WR1) that Cloudflare serves to RSA-only clients. Update (during apply): the TLS client is pinned to RSA suites (`CONFIG_MBEDTLS_KEY_EXCHANGE_ECDHE_ECDSA is not set`) because the classic ESP32 has no ECC accelerator and the Wokwi sim's emulated CPU is ~100x slower than silicon — a software-ECDSA handshake (~16 sim-seconds) exceeds Cloudflare's ~13 s handshake timeout, while RSA verification rides the hardware MPI and completes in ~4 sim-seconds. (`MBEDTLS_ECDSA_C` itself is force-selected by the WiFi component and cannot be compiled out.) The PEM file keeps a trailing NUL byte for `X509::pem_until_nul`. If Coinbase rotates CAs, update the embedded cert.

7. **Task layout:** FreeRTOS task A = display/UI (e-paper redraw + refresh only on content change); task B = fetch loop (WiFi connect → fetch → signal UI → sleep 60 s), communicating via a shared value + state (e.g. `Arc<Mutex<AppState>>` or a channel). Error states are drawn by the UI task, so network hiccups never corrupt rendering.

8. **Config via env/`sdkconfig.defaults`:** WiFi SSID/password and the fetch refresh interval (`FETCH_INTERVAL_SECS`, default 60 s) come from environment variables at build time (`esp-idf` convention) so the same binary works for Wokwi (gateway WiFi) and home hardware by changing config, not code.

## Risks / Trade-offs

- [Wokwi networking needs the private IoT Gateway (account token)] → set up `WOKWI_CLI_TOKEN` in Phase 2; the blink milestone verifies gateway connectivity before any display work
- [Xtensa cold builds are slow (~1 min+), ESP-IDF download is large] → acceptable one-time cost; warm incremental builds are fine
- [TLS heap pressure on classic ESP32 (~520 KB RAM)] → response payload is ~100 bytes; keep reqwest buffers minimal; if tight, drop to raw `esp-mbedtls` HTTPS later without changing specs
- [Coinbase CA rotation breaks pinned root] → error state is visible on screen; update embedded cert is a one-line config change
- [Wokwi sim time ≠ real time] → 60 s refresh cadence is unaffected in practice; verify interval on real hardware during flashing phase
- [E-paper refresh wear/ghosting from frequent full refreshes] → firmware refreshes only when content changes; partial refresh (`update_partial_frame` + `RefreshLut::Quick`) available as a later optimization; FETCH_INTERVAL_SECS configurable if cadence must drop
- [`epd-waveshare` git-master dependency (the plain-2.7 module is unreleased)] → pin the revision in Cargo.lock; small, self-contained driver if we ever need to vendor it
- [E-paper `BUSY` blocking in simulation (no panel to answer)] → hold BUSY idle in `diagram.json`; hardware verifies real busy timing

## Migration Plan

Not applicable (greenfield). Rollback = delete the crate; nothing existing depends on it.

## Open Questions

None. The panel (Good Display GDEY027T91) and its controller are confirmed; display wiring is fixed by the driver board's e-paper connector.
