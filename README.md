# sp32-demo1

ESP32 (ESP-WROOM-32E) firmware in Rust: displays the live BTC-USD spot price from Coinbase on an ILI9341 TFT. Development happens in the [Wokwi simulator](https://wokwi.com) with real HTTPS traffic — no hardware required until the final flash.

## Toolchain setup (one-time)

Requirements (Fedora): `sudo dnf install -y openssl-devel pkgconf-pkg-config`

> If `espup` fails building OpenSSL, run the install with `OPENSSL_NO_VENDOR=1 cargo install espup --locked` (needs `openssl-devel` + `pkg-config`).

```bash
# 1. Xtensa Rust fork + toolchain components (Rust fork named "esp")
cargo install espup ldproxy
espup install

# 2. Environment variables (every new shell, or add to your profile)
. ~/export-esp.sh
```

The project pins `channel = "esp"` via `rust-toolchain.toml`, so plain `cargo build` inside this directory already uses the right toolchain after sourcing `export-esp.sh`.

## Build

```bash
cargo build            # dev build (ELF in target/xtensa-esp32-espidf/debug/)
cargo build --release  # release build (used for Wokwi + flashing)
```

First build downloads ESP-IDF automatically (several GB, ~10 min). Subsequent builds are incremental.

## Simulate in Wokwi

See [Simulate](#simulate) below.

<!-- replaced -->

## Simulate

Requires a Wokwi account token (free): create one at https://wokwi.com/dashboard/ci, then:

```bash
export WOKWI_CLI_TOKEN="wok_..."   # or source ~/.config/wokwi/token.env
./scripts/sim.sh                    # build + simulate, Ctrl+C to stop
```

`scripts/sim.sh` = `cargo build --release` + esptool image refresh + `wokwi-cli`. Any arguments are passed to `wokwi-cli`, e.g. `./scripts/sim.sh --timeout 60000 --expect-text "WiFi connected"`.

Useful flags: `--serial-log-file <file>`, `--expect-text <text>` (CI-style pass/fail), `--timeout <ms>`.

Note: Wokwi loads the app through ESP-IDF's `flasher_args.json`; because `cargo` re-links the Rust ELF after ESP-IDF's cmake step, `scripts/prepare-wokwi.sh` regenerates the app image (`esptool elf2image`) from the current ELF before each run.

## Layout

- `src/main.rs` — firmware: WiFi connect, ILI9341 UI task, Coinbase fetch task (shared state via `Arc<Mutex<AppState>>`)
- `price/` — pure-std crate: spot-response parsing (`serde_json`) + `$67,432` formatting; unit-tested on the host
- `price/tests/fixtures/spot_price.json` — recorded real API response used by the tests
- `certs/coinbase-root-ca.pem` — embedded TLS trust anchor (GTS Root R4, self-signed, from pki.goog); keep the trailing NUL byte — `X509::pem_until_nul` requires it
- `scripts/` — `prepare-wokwi.sh` (build + flash-image refresh), `sim.sh` (one-command simulation)

## Tests

The `price` crate is pure std, so tests run on the host (the repo pins the xtensa target in `.cargo/config.toml`, hence the explicit host target):

```bash
cargo +stable test -p price --target x86_64-unknown-linux-gnu
```

## Known Wokwi limitation

The Wokwi cloud simulator's virtual network link is slow: the Coinbase TLS handshake (its ~3.5 KB ECDSA certificate chain alone transfers for ~10 sim-seconds) can be reset by the gateway/server before completing. On real hardware the same handshake takes well under a second. When a sim run shows `ERROR` on the display with `Price update failed: ESP_ERR_HTTP_CONNECT` in serial, the firmware is fine — the fetch retries every 10 s, and a sim run under lighter cloud load may succeed.

