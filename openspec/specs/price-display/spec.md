# price-display Specification

## Purpose

Renders the application state on the Good Display GDEY027T91 2.7" monochrome e-paper panel (264×176 landscape) over SPI: the current BTC-USD price as large readable text, updated after every successful refresh, with a visible error state when data is unavailable.

## Requirements

### Requirement: Price rendering
The firmware SHALL render the current BTC-USD price as large text filling a significant portion of the 264×176 display, legible from arm's length.

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
The firmware SHALL initialize the GDEY027T91 (SSD1680 controller) over SPI using the Waveshare E-Paper ESP32 Driver Board's fixed e-paper wiring (SCK=GPIO13, MOSI=GPIO14, CS=GPIO15, DC=GPIO27, RST=GPIO26, BUSY=GPIO25) so that drawing commands produce a visible frame after a refresh cycle.

> Amendment (during apply, display pivot): the original ILI9341 TFT target was replaced by the e-paper panel. The physical TFT module never produced a frame despite verified bus activity, power, and reset at the module; the e-paper panel and driver board are a native pair (flex connector, no external wiring) and match the ticker use case (bistable, daylight readable, no backlight). The SPI bus and control pins are therefore the driver board's fixed e-paper pins, and no backlight exists.

#### Scenario: Display comes up
- **WHEN** the firmware boots
- **THEN** the display is initialized and the waiting state becomes visible on the panel after the initial refresh

### Requirement: Refresh discipline
The firmware SHALL only trigger an e-paper refresh cycle when the content to display has changed (state transition or new price string), so that an unchanged price does not cause unnecessary flashing or panel wear.

#### Scenario: Unchanged price
- **WHEN** a fetch returns the same price that is already displayed
- **THEN** no new refresh cycle is triggered

#### Scenario: Changed content
- **WHEN** the price string or app state changes
- **THEN** a refresh cycle updates the panel to the new content
