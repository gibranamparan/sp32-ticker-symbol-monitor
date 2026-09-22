# ticker-ui Specification

## Purpose

Defines the ticker screen frame: how the e-paper display is divided, what appears in each region, the text hierarchy, and the freshness information (last-update timestamp) shown alongside the price.

## Requirements

### Requirement: Ticker frame layout

The display SHALL be divided into three equal-height rows, each occupying one third of the screen height:

- row 1: the Bitcoin logo and the pair label (`BTC/USD`) on one line, centered as a group
- row 2: the current price, centered horizontally
- row 3: the last-update timestamp as standard-size text, aligned to the end of the row

#### Scenario: Frame is rendered

- **WHEN** the display shows a price
- **THEN** the screen contains three equal-height rows with the logo and pair label centered in the first row, the centered price in the second row, and the end-aligned timestamp in the third row

#### Scenario: Rows are equal

- **WHEN** the rendered frame is measured
- **THEN** each of the three rows is one third of the display height (within a few pixels of rounding)

### Requirement: Text hierarchy

The price SHALL be the largest text on the screen, the pair label the second largest, and every other text element the standard body size; this hierarchy SHALL be consistent across all display states.

#### Scenario: Sizes are ordered

- **WHEN** the rendered text elements are compared
- **THEN** the price (h1) is larger than the pair label (h2), which is larger than the timestamp and any other body text (1em)

### Requirement: Last update timestamp

The display SHALL show when the price was last successfully updated, formatted as `dd-MMM-yyyy HH:mm` in 24-hour time, expressed in the configured display timezone (a UTC offset).

#### Scenario: Timestamp after a successful update

- **WHEN** a price fetch succeeds
- **THEN** the timestamp reflects the moment of that successful update in the specified format and the configured timezone offset (e.g. `21-Sep-2026 14:06` for `UTC-6` when UTC is `20:06`)

#### Scenario: No successful update yet

- **WHEN** no price has been successfully fetched yet
- **THEN** the frame renders with a placeholder timestamp instead of a fabricated time

#### Scenario: Timestamp advances on refresh

- **WHEN** a later fetch succeeds
- **THEN** the displayed timestamp advances to the new update time

### Requirement: Timezone configuration

The display timezone SHALL be supplied as explicit configuration (a UTC offset such as `UTC-6`); the firmware SHALL NOT silently assume a timezone.

#### Scenario: Missing timezone configuration

- **WHEN** the timezone configuration is not provided
- **THEN** the build fails rather than producing a binary

#### Scenario: Malformed timezone configuration

- **WHEN** the configured value is not a valid UTC offset
- **THEN** the firmware fails to initialize and reports the problem on the serial log

#### Scenario: Timezone applies to the displayed time only

- **WHEN** a timezone offset is configured
- **THEN** the on-screen timestamp is shifted by that offset while the underlying clock and fetch scheduling remain UTC-based

### Requirement: Network time synchronization

The device clock SHALL be synchronized from the network so timestamps reflect real time; the timestamp SHALL NOT be derived from device uptime.

#### Scenario: Clock syncs after connecting

- **WHEN** the device has network connectivity
- **THEN** its clock is synchronized from an NTP source and subsequent timestamps reflect real UTC time

#### Scenario: Sync unavailable

- **WHEN** time synchronization is unavailable
- **THEN** the display still renders the frame and shows the placeholder timestamp rather than an incorrect time, and synchronization is retried

### Requirement: Wireframe is the design reference

The committed HTML wireframe SHALL be the design reference for the ticker screen; the firmware rendering SHALL match its structure — row contents, alignment, and text hierarchy.

#### Scenario: Rendering matches the wireframe

- **WHEN** the approved wireframe and the e-paper output are compared
- **THEN** the row structure, alignment, and relative text sizes correspond
