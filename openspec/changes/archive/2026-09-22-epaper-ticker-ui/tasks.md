# Tasks

## 1. Wireframe (design reference)

- [x] 1.1 Create the grayscale logo asset `wireframes/assets/bitcoin-logo-gray.svg` from the existing color SVG (desaturated fills) — verify: the file exists and renders in grayscale in a browser
- [x] 1.2 Create `wireframes/wireframe.html` with static values: three equal-height rows (row 1: grayscale logo + `<h2>BTC/USD</h2>` inline, centered as a group; row 2: centered `<h1>` price, bottom-aligned in its row; row 3: end-aligned standard-size `dd-MMM-yyyy HH:mm` timestamp, bottom-aligned), with `h1` largest, `h2` second largest, everything else `1em`, laid out at the panel's aspect ratio — verify: opens in a browser and shows the specified layout and hierarchy
- [x] 1.3 Obtain user approval of the wireframe before any firmware work — verify: user confirms the layout (later tasks depend on this gate)

## 2. Date-time formatting (host-testable)

- [x] 2.1 Add UTC `dd-MMM-yyyy HH:mm` formatting for an epoch timestamp in the pure `price` crate with unit tests (month abbreviations, zero-padding, midnight/noon edges) — verify: `cargo +stable test -p price --target x86_64-unknown-linux-gnu`

## 3. Time synchronization

- [x] 3.1 Start SNTP after WiFi is up (`EspSntp`, UTC) and keep retrying while unsynchronized — verify: serial shows sync status; timestamps switch from placeholder to real UTC time
- [x] 3.2 Extend shared `AppState` with the last successful price-update time; show the placeholder until both a sync and a successful fetch have happened — verify: serial log and frame show the placeholder before the first successful update
- [x] 3.3 Add the required `TIMEZONE` build-time variable (UTC offset, e.g. `UTC-6`): offset parsing lives in the pure `price` crate with host tests, a missing variable fails the build, a malformed one fails initialization, and the offset applies to the displayed timestamp only — verify: `cargo +stable test -p price --target x86_64-unknown-linux-gnu` plus the on-screen time shifted on hardware

## 4. Frame rendering on the e-paper

- [x] 4.1 Convert the logo to a 1-bit bitmap (35×35) embedded as a const and drawable with `embedded-graphics` — verify: the logo appears centered with the pair label in row 1 on the panel
- [x] 4.2 Rework `draw_frame` into the three equal-height rows (logo + `h2` pair label; centered `h1` price; end-aligned `1em` timestamp), with all font sizes in one constants block — verify: panel structure and hierarchy match the approved wireframe
- [x] 4.3 Keep refresh-on-content-change so the frame updates whenever the price or timestamp changes — verify: one refresh observed per successful fetch cycle (~60 s)
- [x] 4.4 Verify on hardware: layout, contrast, legibility, timestamp accuracy in the configured timezone, and correspondence with the wireframe — verify: side-by-side comparison with `wireframes/wireframe.html`
- [x] 4.5 Run the simulation to confirm boot/WiFi/fetch/state still work headless — verify: `./scripts/sim.sh` logs spot prices and refreshes

## 5. Documentation

- [x] 5.1 Update the README (wireframe reference, SNTP note, layout/font mapping) — verify: README documents the wireframe-first workflow and the new on-screen elements
