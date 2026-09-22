# Design

## Context

See `proposal.md` — Why. Current state that shapes the approach:

- The panel is a 264×176 landscape e-paper (SSD1680, 1-bit black/white); the firmware draws with `embedded-graphics` into a full-frame buffer (`epd-waveshare`, `Display2in7`) and refreshes only when content changes.
- Fonts available: `profont` bitmap sizes (7/9/10/12/14/18/24 pt); there is no vector font and no text scaling in `embedded-graphics`, so the "h1/h2/1em" contract must map to concrete bitmap sizes (and the existing 2× pixel doubler for the largest text).
- The device currently has **no real clock** (uptime-based only); the new timestamp requirement forces a time source.
- The 24-pin logo is a color SVG; the panel is 1-bit, so it becomes a monochrome bitmap.

## Goals / Non-Goals

**Goals:**
- A browser-reviewable wireframe that is the single source of truth for the layout, approved before firmware work.
- A faithful e-paper rendering of that wireframe within the panel's constraints (1-bit, bitmap fonts, no anti-aliasing).
- Real time on screen, without fabricating values when time is unknown.

**Non-Goals:**
- Partial/fast-refresh optimization (full refresh per change is acceptable; revisit later).
- New data on screen (24 h change, charts, battery) — unchanged from the existing scope.
- Touch/interaction, multiple currencies, animated content.

## Decisions

1. **Wireframe-first, in plain HTML with static values.** A single `wireframes/wireframe.html` (no build step, no CSS framework) renders the frame at the panel's aspect ratio with static content, so layout and hierarchy are reviewed in a browser in seconds instead of flashing firmware. The wireframe doubles as the visual reference for the spec's "wireframe is the design reference" requirement.

2. **Logo: grayscale asset for the wireframe, 1-bit bitmap for the panel.** The wireframe uses a grayscale copy of the SVG (`wireframes/assets/bitcoin-logo-gray.svg`, fills desaturated) so it reads as a monochrome element there; for the firmware, the logo is converted once (host script) into a small 1-bit bitmap (35×35, grown 10% from the first hardware review) embedded as a `const` array and drawn pixel-by-pixel with `embedded-graphics`. Alternatives: drawing the logo with primitives (worse fidelity), rendering text "₿" (no such glyph in profont).

3. **Typography mapping to the panel.** `h1` = price = the existing doubled 24 pt profont (≈48 px tall, visually dominant); `h2` = pair label = 24 pt; `1em` = timestamp/body = 12 pt. This preserves "h1 > h2 > 1em" while fitting 264 px width (7-character price at 2× ≈ 200 px; pair label and timestamp fit comfortably). Font sizes live in one constant block so the ratio can be tuned in one place.

4. **Time source: ESP-IDF SNTP (`esp_idf_svc::sntp::EspSntp`), UTC internally; display timezone from `TIMEZONE`.** The wireframe's `dd-MMM-yyyy HH:mm` format needs absolute time; uptime-derived time would be a lie. SNTP starts after WiFi is up and runs continuously; the clock stays UTC and the configured offset is applied only when formatting the on-screen timestamp (and the log line). `TIMEZONE` is a required build-time variable (forms `UTC`, `UTC-6`, `UTC+5:30`, `-6`, `+05:30`): a missing value fails the build (`env!`), a malformed one fails initialization with a clear panic, so the firmware never silently assumes a zone. An IANA timezone database was rejected (no tzdata on the device, large flash cost for a single offset); an RTC module was rejected (extra hardware, not present). Update (during apply, after hardware review): the first version displayed UTC and the review asked for a configurable zone — hence the offset configuration.

5. **Frame content and refresh interaction.** `AppState` gains the last-update timestamp (formatted string or epoch). The UI redraws when the frame content changes — since the timestamp advances on every successful fetch, the panel now refreshes about once per fetch cycle even if the price is unchanged. This is accepted: e-paper panels tolerate far more cycles than a ticker will produce, and the alternative (only refresh on price change) would leave a visibly stale timestamp. Partial refresh of just the timestamp region is a future optimization.

6. **Date formatting is host-testable.** `dd-MMM-yyyy HH:mm` formatting is pure logic; it goes into the existing pure-Rust `price` crate (or a sibling pure module) with unit tests, matching how price parsing/formatting is already tested on the host rather than on-device.

## Risks / Trade-offs

- [1-bit logo at 35×35 loses detail] → acceptable for a small brand mark; verified during hardware bring-up and adjusted (grew 10% after review).
- [Bitmap font sizes are coarse; "equal rows" may not fit text exactly] → rows are one third each with content vertically centered in its row; if the h1 price overflows a row, shrink the price or the row padding (layout constant, one-line change).
- [SNTP unavailable on some networks] → placeholder timestamp + retry; the price still renders and updates (time is additive, not blocking).
- [More frequent full refreshes because of the timestamp] → accepted (see decision 5); monitor for ghosting; partial refresh later if needed.
- [Wireframe/firmware drift] → the spec makes the wireframe the reference; verify side by side during hardware bring-up.
