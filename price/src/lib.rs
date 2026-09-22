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

/// The configured UTC offset string could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetError;

impl core::fmt::Display for OffsetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "invalid UTC offset (expected forms: UTC, UTC-6, UTC+5:30, -6, +05:30)"
        )
    }
}

impl std::error::Error for OffsetError {}

/// Parse a UTC offset specification into seconds east of UTC.
///
/// Accepted forms: `UTC`, `UTC-6`, `UTC+5`, `UTC+5:30`, `-6`, `+05:30`
/// (the `UTC` prefix is optional and case-insensitive). Hours are limited to
/// 14 and minutes to 59; anything else is an error.
pub fn parse_utc_offset(spec: &str) -> Result<i32, OffsetError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(OffsetError);
    }

    let rest = if trimmed.len() >= 3 && trimmed[..3].eq_ignore_ascii_case("UTC") {
        &trimmed[3..]
    } else {
        trimmed
    };
    if rest.is_empty() {
        return Ok(0);
    }

    let (sign, digits) = match rest.as_bytes()[0] {
        b'+' => (1, &rest[1..]),
        b'-' => (-1, &rest[1..]),
        _ => return Err(OffsetError),
    };

    let (hours_str, minutes_str) = match digits.split_once(':') {
        Some((h, m)) => (h, Some(m)),
        None => (digits, None),
    };
    if hours_str.is_empty() || !hours_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(OffsetError);
    }
    let hours: i32 = hours_str.parse().map_err(|_| OffsetError)?;

    let minutes: i32 = match minutes_str {
        Some(m) => {
            if m.is_empty() || m.len() > 2 || !m.bytes().all(|b| b.is_ascii_digit()) {
                return Err(OffsetError);
            }
            m.parse().map_err(|_| OffsetError)?
        }
        None => 0,
    };

    if hours > 14 || minutes > 59 {
        return Err(OffsetError);
    }
    Ok(sign * (hours * 3_600 + minutes * 60))
}

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

const MONTH_ABBREVIATIONS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Format an epoch timestamp (seconds since 1970-01-01T00:00:00Z) as
/// `dd-MMM-yyyy HH:mm` in UTC, e.g. `1790021160` -> `21-Sep-2026 20:06`.
pub fn format_timestamp(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs_of_day = epoch_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    format!(
        "{day:02}-{}-{year:04} {hour:02}:{minute:02}",
        MONTH_ABBREVIATIONS[(month - 1) as usize]
    )
}

/// Days since the Unix epoch to a civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}
