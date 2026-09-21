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

## Flash to hardware

Create your credentials file once (gitignored, never committed):

```bash
cp toml.production.example toml.production   # then edit in your SSID/password
```

Build, flash, and monitor over USB — with those credentials overriding the Wokwi ones:

```bash
./scripts/hw.sh                 # build + flash + serial monitor (Ctrl+C to exit)
./scripts/hw.sh --no-monitor    # build + flash only, exit when done
```

This is the only path that reads `toml.production` (passed to cargo as a `--config` fragment, which takes precedence over `.cargo/config.toml`). Simulation builds (`scripts/sim.sh`, plain `cargo build`) always use `WIFI_SSID`/`WIFI_PASS` from `.cargo/config.toml` (`Wokwi-GUEST`). A shell-exported `WIFI_SSID`/`WIFI_PASS` overrides everything (cargo `[env]` semantics).

`FETCH_INTERVAL_SECS` (default `10`, seconds between price refreshes) follows the same precedence: set it in `toml.production` or the shell to change the cadence for a given build.

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

## TLS notes (ESP32 classic)

The classic ESP32 has a hardware RSA accelerator but **no ECC/ECDSA accelerator**. Cloudflare (api.coinbase.com's edge) kills TLS handshakes that take more than ~13 s, and a software-ECDSA handshake — which is what gets negotiated by default — takes ~16 sim-seconds in Wokwi (the emulated CPU is ~100x slower than real silicon). The firmware therefore pins the TLS client to RSA suites (`CONFIG_MBEDTLS_KEY_EXCHANGE_ECDHE_ECDSA is not set` in `sdkconfig.defaults`): the server then serves its RSA chain (`coinbase.com` ← GTS WR1), verified against the embedded GTS Root R1 anchor using the hardware MPI, completing in ~4 sim-seconds.

Gotchas if you change mbedtls config: disabling a symbol in `sdkconfig.defaults` requires the `# CONFIG_X is not set` form (a `CONFIG_X=n` line is ignored), and the `esp-idf-sys` build script may need `cargo clean --release -p esp-idf-sys` to actually pick the change up.

