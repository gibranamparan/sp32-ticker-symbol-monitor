## Purpose

Renders the application state on the 240×320 ILI9341 TFT over SPI: the current BTC-USD price as large readable text, updated after every successful refresh, with a visible error state when data is unavailable.

## ADDED Requirements

### Requirement: Price rendering
The firmware SHALL render the current BTC-USD price as large text filling a significant portion of the 240×320 display, legible from arm's length.

#### Scenario: First successful fetch
- **WHEN** the first price fetch succeeds
- **THEN** the display shows the formatted price (e.g. `$67,432`) as the dominant on-screen element

#### Scenario: Price update
- **WHEN** a subsequent fetch returns a different price
- **THEN** the displayed price is replaced with the new value, leaving no stale digits or artifacts

### Requirement: Error state display
The firmware SHALL show a distinct error state on the display when it has no price data (startup failure) or when a fetch fails and no price is known.

#### Scenario: No data at startup
- **WHEN** the display is initialized before the first successful price fetch
- **THEN** the display shows a connecting/waiting state rather than a blank or uninitialized screen

#### Scenario: Fetch failure with no known price
- **WHEN** a fetch fails and no previous price is known
- **THEN** the display shows a visible error state instead of the price

#### Scenario: Fetch failure with known price
- **WHEN** a fetch fails but a previously fetched price is known
- **THEN** the display MAY continue showing the last known price; it SHALL NOT show a corrupt or partially rendered frame

### Requirement: Display initialization
The firmware SHALL initialize the ILI9341 over SPI using the standard 38-pin DevKit wiring (SCK=GPIO18, MISO=GPIO19, MOSI=GPIO23, CS=GPIO5, DC=GPIO2, RST=GPIO4, backlight=GPIO21) so that drawing commands produce visible pixels.

#### Scenario: Display comes up
- **WHEN** the firmware boots
- **THEN** the display is initialized and any drawn content becomes visible, including in the Wokwi virtual display
