# sp32-demo1

ESP32 (ESP-WROOM-32E) firmware in Rust: displays the live BTC-USD spot price from Coinbase on a Good Display GDEY027T91 2.7" e-paper panel (264×176, SSD1680) attached to a Waveshare E-Paper ESP32 Driver Board. Development happens in the [Wokwi simulator](https://wokwi.com) with real HTTPS traffic — no hardware required until the final flash.

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

Note: Wokwi has no matching e-paper part. The simulation validates build, WiFi, HTTPS fetch, and state updates — the e-paper `BUSY` line is held idle in `diagram.json` so the display driver does not block. Visible display output is verified on the physical panel.

## Display (hardware)

The GDEY027T91 hangs off the driver board's fixed e-paper connector — no external wiring. Firmware pin map: `SCK=GPIO13, MOSI=GPIO14, CS=GPIO15, DC=GPIO27, RST=GPIO26, BUSY=GPIO25` (the pins Waveshare hard-wires to the 24-pin flex). The UI rotates the panel to 264×176 landscape and refreshes only when the displayed content changes (e-paper wear + flashing). Driver: `epd-waveshare` (pinned git revision, `epd2in7_v2` module).

Panel bring-up pattern (stripes, one full refresh): build with `PANEL_TEST=1`, e.g. `PANEL_TEST=1 ./scripts/hw.sh`.

## UI

The ticker screen is designed wireframe-first: `wireframes/wireframe.html` is the design reference and the firmware mirrors it.

- Three equal-height rows: (1) grayscale logo + `BTC/USD` centered as a group, (2) price centered horizontally and bottom-aligned in its row, (3) last-update timestamp end-aligned and bottom-aligned.
- Typography: `h1` price = 24 pt profont doubled (≈48 px), `h2` pair label = 24 pt, body (`1em`) = 12 pt.
- The last-update time is real time synchronised via SNTP (clock kept in UTC) and displayed in the configured timezone as `dd-MMM-yyyy HH:mm`; before the clock syncs the frame shows `--` instead of a fabricated time.
- The logo is a 35×35 1-bit bitmap generated from the SVG asset: `.embuild/espressif/python_env/idf5.5_py3.12_env/bin/python scripts/convert-logo.py` (requires `cairosvg` in the IDF venv).

To view the wireframe: open `wireframes/wireframe.html` in a browser, or serve the repo locally (e.g. `python3 -m http.server`) and open it over HTTP.

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

`FETCH_INTERVAL_SECS` (default `60`, seconds between price refreshes) follows the same precedence: set it in `toml.production` or the shell to change the cadence for a given build.

`TIMEZONE` is the display timezone as a UTC offset (`UTC`, `UTC-6`, `UTC+5:30`, `-6`, `+05:30`) and is **required** — the build fails if the variable is missing entirely, and the firmware refuses to start on a malformed value. The clock itself stays UTC; the offset is applied to the on-screen timestamp only. Repo default: `UTC-6` (see `.cargo/config.toml`).

## Layout

- `src/main.rs` — firmware: WiFi connect, SNTP clock, e-paper UI task (three-row frame, refresh on content change), Coinbase fetch task (shared state via `Arc<Mutex<AppState>>`)
- `src/logo.rs` — generated 1-bit Bitcoin logo bitmap (see `scripts/convert-logo.py`)
- `wireframes/` — the HTML design reference + logo assets
- `price/` — pure-std crate: spot-response parsing (`serde_json`), `$67,432` formatting, and UTC `dd-MMM-yyyy HH:mm` timestamps; unit-tested on the host
- `price/tests/fixtures/spot_price.json` — recorded real API response used by the tests
- `certs/coinbase-root-ca.pem` — embedded TLS trust anchor (GTS Root R1, self-signed, from pki.goog); keep the trailing NUL byte — `X509::pem_until_nul` requires it
- `scripts/` — `prepare-wokwi.sh` (build + flash-image refresh), `sim.sh` (one-command simulation), `hw.sh` (production build + flash)

## Tests

The `price` crate is pure std, so tests run on the host (the repo pins the xtensa target in `.cargo/config.toml`, hence the explicit host target):

```bash
cargo +stable test -p price --target x86_64-unknown-linux-gnu
```

## TLS notes (ESP32 classic)

The classic ESP32 has a hardware RSA accelerator but **no ECC/ECDSA accelerator**. Cloudflare (api.coinbase.com's edge) kills TLS handshakes that take more than ~13 s, and a software-ECDSA handshake — which is what gets negotiated by default — takes ~16 sim-seconds in Wokwi (the emulated CPU is ~100x slower than real silicon). The firmware therefore pins the TLS client to RSA suites (`CONFIG_MBEDTLS_KEY_EXCHANGE_ECDHE_ECDSA is not set` in `sdkconfig.defaults`): the server then serves its RSA chain (`coinbase.com` ← GTS WR1), verified against the embedded GTS Root R1 anchor using the hardware MPI, completing in ~4 sim-seconds.

Gotchas if you change mbedtls config: disabling a symbol in `sdkconfig.defaults` requires the `# CONFIG_X is not set` form (a `CONFIG_X=n` line is ignored), and the `esp-idf-sys` build script may need `cargo clean --release -p esp-idf-sys` to actually pick the change up.

