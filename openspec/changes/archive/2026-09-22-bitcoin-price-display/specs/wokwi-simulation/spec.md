## Purpose

Lets the identical firmware binary run in the Wokwi simulator with real end-to-end HTTPS traffic through the Wokwi IoT Gateway, exercising the build, WiFi, fetch, and state-update behavior of the app. Display output is verified on the physical e-paper panel (Wokwi has no matching e-paper part).

## ADDED Requirements

### Requirement: Same binary in simulator and on hardware
The firmware SHALL be buildable once and runnable both in the Wokwi simulator and on the physical ESP-WROOM-32E, with no source changes between the two targets; target-specific values (WiFi SSID/password, serial behavior) SHALL come from configuration, not code edits.

#### Scenario: Simulated run
- **WHEN** the firmware is launched in Wokwi via the project's simulation configuration
- **THEN** the virtual ESP32 executes the same compiled binary that would be flashed to hardware

#### Scenario: Hardware run
- **WHEN** the same build artifact is flashed to the physical board
- **THEN** the firmware performs the same fetch-and-display behavior as in the simulator

### Requirement: Real network access in simulation
The simulated firmware SHALL perform real HTTPS requests to the Coinbase API through the Wokwi IoT Gateway, with TLS handled by the firmware itself; the simulator SHALL NOT substitute mocked or replayed API responses in the application path.

#### Scenario: End-to-end fetch in simulator
- **WHEN** the firmware running in Wokwi performs a price fetch
- **THEN** a real TLS-secured request reaches api.coinbase.com and the fetched price appears in the application state and serial log

### Requirement: Simulated application basics
The Wokwi diagram SHALL keep the firmware runnable without a physical display: the e-paper `BUSY` input SHALL be held at its idle level so the display driver does not block, and the simulation SHALL exercise boot, WiFi connect, HTTPS fetch, and state updates. Virtual rendering of the e-paper panel is not required (Wokwi provides no matching e-paper part); visible output is verified on hardware.

#### Scenario: Firmware runs in simulator
- **WHEN** the firmware boots in Wokwi with the BUSY line held idle
- **THEN** it reaches the fetch loop, logs prices, and updates application state without blocking on the display

### Requirement: Simulation entry point
The project SHALL provide a single command entry point (via the Wokwi CLI) to build and launch the simulation, and the simulation configuration SHALL be committed to the repository.

#### Scenario: One-command simulation
- **WHEN** a developer runs the project's simulation command from a fresh checkout with the toolchain installed
- **THEN** the firmware builds and starts running in the Wokwi simulator with networking enabled
