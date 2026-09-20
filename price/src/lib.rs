//! Parsing and formatting of the Coinbase spot-price response.
//!
//! Pure std code, no ESP dependencies, so it can be unit-tested on the host
//! (`cargo +stable test -p price`) while the firmware links the same logic.

use serde::Deserialize;

/// The response body could not be parsed into a price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError;

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "missing or malformed price data")
    }
}

impl std::error::Error for ParseError {}

#[derive(Deserialize)]
struct SpotResponse {
    data: SpotData,
}

#[derive(Deserialize)]
struct SpotData {
    amount: String,
}

/// Parse a `GET /v2/prices/BTC-USD/spot` response body into the price value.
///
/// Anything that does not yield a finite numeric price is an error, so
/// malformed responses surface as failures instead of garbage on screen.
pub fn parse_spot_response(body: &str) -> Result<f64, ParseError> {
    let resp: SpotResponse = serde_json::from_str(body).map_err(|_| ParseError)?;
    let amount = resp.data.amount.trim();
    match amount.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(ParseError),
    }
}

/// Format a price for display: whole dollars with thousands separators,
/// e.g. `67432.15` -> `$67,432` (the format pinned in the spec).
pub fn format_price(value: f64) -> String {
    let rounded = value.round();
    let neg = rounded < 0.0;
    let digits = rounded.abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 2);
    out.push('$');
    if neg {
        out.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
