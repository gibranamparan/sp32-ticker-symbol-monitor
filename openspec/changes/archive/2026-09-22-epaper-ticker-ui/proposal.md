# Proposal

## Why

The e-paper screen currently shows a bare price with a small label — no identity, no freshness information, no deliberate visual hierarchy. Firmware iteration on the panel is slow, so the screen is designed first as a static HTML wireframe (fast to tweak, reviewable in a browser) and only then reproduced on the e-paper.

## What Changes

- New design reference: `wireframes/wireframe.html` (static values) plus a grayscale Bitcoin logo asset.
- Layout: the screen is split into **three equal-height rows**
  - row 1 — logo + pair name (`BTC/USD`) inline, centered as a group
  - row 2 — current price in `h1`, **centered**
  - row 3 — last-update date-time (`dd-MMM-yyyy HH:mm`, 24 h) as regular text, aligned to the **end**
- Typography contract: `h1` is the largest text on screen, `h2` the second largest, all other text is `1em`.
- New on-screen **last-update timestamp**, which requires a real device clock → network time sync (SNTP), with defined behavior before a first successful sync; displayed in a required, explicitly configured timezone (a `TIMEZONE` UTC-offset build-time variable — the build fails if it is missing).
- After wireframe approval, the firmware reproduces the approved layout on the e-paper (logo bitmap, pair label, price, timestamp), keeping the existing "refresh only when displayed content changes" discipline.
- No changes to fetching, TLS, WiFi, or configuration behavior.

## Capabilities

### New Capabilities
- `ticker-ui`: the ticker screen frame — three-row layout, typography hierarchy, logo + pair label, price presentation, last-update timestamp, and the network time synchronization that supports it. The approved HTML wireframe is the design reference for the firmware rendering.

### Modified Capabilities

(none — `price-display` price/error/waiting requirements and `price-fetching` cadence are unchanged; this change adds the frame layout around the existing content)

## Impact

- New files: `wireframes/wireframe.html`, grayscale logo asset under `wireframes/assets/`.
- Firmware: UI drawing rework (three-row frame), logo converted to a 1-bit bitmap, date-time formatting, SNTP-based clock, a required `TIMEZONE` UTC-offset configuration (build fails if absent, boot fails if malformed), and `AppState` gaining the last-update timestamp.
- Refresh behavior trade-off: the timestamp changes on every successful fetch, so the panel refreshes about once per fetch cycle (~60 s) even when the price is unchanged — the "unchanged price" case no longer suppresses a refresh, because the frame content genuinely changes.
- Simulation keeps covering build/WiFi/fetch/state (no e-paper part in Wokwi); the layout itself is verified on the physical panel.
