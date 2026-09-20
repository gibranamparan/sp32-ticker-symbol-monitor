## Purpose

Provides the firmware with live market data: joining a WiFi network and retrieving the current BTC-USD spot price from the Coinbase public API over HTTPS, so the display always shows a recent price.

## ADDED Requirements

### Requirement: WiFi station connection
The firmware SHALL connect to the configured WiFi access point in station mode using credentials provided at build/run time (SSID and password).

#### Scenario: Successful connection
- **WHEN** the firmware starts with valid WiFi credentials in range
- **THEN** it establishes a WiFi station connection and obtains network access

#### Scenario: Connection failure
- **WHEN** the WiFi credentials are wrong or the access point is unreachable
- **THEN** the firmware retries the connection and reports the failure state to the display capability

### Requirement: Spot price retrieval
The firmware SHALL fetch the current BTC-USD spot price from the Coinbase public API endpoint (`GET /v2/prices/BTC-USD/spot`) over HTTPS, without requiring an API key.

#### Scenario: Successful fetch
- **WHEN** the API responds with HTTP 200 and a JSON body containing the spot price
- **THEN** the firmware extracts the numeric price value (e.g. `"amount":"67432.15"`) and makes it available to the display capability

#### Scenario: TLS-secured transport
- **WHEN** the firmware connects to the API
- **THEN** the connection uses TLS with certificate validation against an embedded trusted root CA, and non-TLS transport SHALL NOT be accepted

#### Scenario: API error or timeout
- **WHEN** the API request fails, times out, or returns a non-success status
- **THEN** the firmware reports the failure state to the display capability and keeps its last known price unchanged

### Requirement: Periodic refresh
The firmware SHALL refresh the spot price every 10 seconds while running.

#### Scenario: Steady-state refresh
- **WHEN** the firmware has been running for multiple minutes
- **THEN** a new price fetch is initiated at approximately 10-second intervals

#### Scenario: Refresh after failure
- **WHEN** a fetch attempt fails
- **THEN** the next attempt is made at the next refresh interval and previously displayed content is not corrupted by the failure

### Requirement: Price parsing correctness
The firmware SHALL parse the API response's price field into a numeric value formatted with a thousands separator for display, and SHALL handle malformed responses as failures rather than displaying garbage.

#### Scenario: Valid response parsed
- **WHEN** the API returns a well-formed JSON body with the price field
- **THEN** the firmware produces a human-readable price string (e.g. `$67,432`)

#### Scenario: Malformed response rejected
- **WHEN** the API returns a body that does not contain a parseable price
- **THEN** the firmware treats the fetch as failed and reports the failure state
