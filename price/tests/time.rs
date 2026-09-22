use price::{format_timestamp, parse_utc_offset, OffsetError};

#[test]
fn formats_the_unix_epoch() {
    assert_eq!(format_timestamp(0), "01-Jan-1970 00:00");
}

#[test]
fn formats_the_last_minute_of_a_day() {
    assert_eq!(format_timestamp(86_399), "01-Jan-1970 23:59");
}

#[test]
fn formats_midnight_of_the_next_day() {
    assert_eq!(format_timestamp(86_400), "02-Jan-1970 00:00");
}

#[test]
fn formats_a_realistic_ticker_sample() {
    // 2026-09-21 20:06 UTC - the wireframe's static value
    assert_eq!(format_timestamp(1_790_021_160), "21-Sep-2026 20:06");
}

#[test]
fn formats_a_leap_day() {
    // 2024-02-29 12:00 UTC
    assert_eq!(format_timestamp(1_709_208_000), "29-Feb-2024 12:00");
}

#[test]
fn formats_a_year_boundary_across_the_century() {
    assert_eq!(format_timestamp(946_684_740), "31-Dec-1999 23:59");
    assert_eq!(format_timestamp(946_684_800), "01-Jan-2000 00:00");
}

#[test]
fn formats_times_before_the_epoch() {
    assert_eq!(format_timestamp(-1), "31-Dec-1969 23:59");
}

#[test]
fn parses_bare_utc() {
    assert_eq!(parse_utc_offset("UTC"), Ok(0));
}

#[test]
fn parses_hour_offsets() {
    assert_eq!(parse_utc_offset("UTC-6"), Ok(-21_600));
    assert_eq!(parse_utc_offset("UTC+5"), Ok(18_000));
    assert_eq!(parse_utc_offset("UTC+0"), Ok(0));
}

#[test]
fn parses_minute_offsets() {
    assert_eq!(parse_utc_offset("UTC+5:30"), Ok(19_800));
    assert_eq!(parse_utc_offset("UTC-06:00"), Ok(-21_600));
    assert_eq!(parse_utc_offset("UTC-6:00"), Ok(-21_600));
}

#[test]
fn utc_prefix_is_optional_and_case_insensitive() {
    assert_eq!(parse_utc_offset("-6"), Ok(-21_600));
    assert_eq!(parse_utc_offset("+05:30"), Ok(19_800));
    assert_eq!(parse_utc_offset("utc+1"), Ok(3_600));
    assert_eq!(parse_utc_offset("  UTC-6  "), Ok(-21_600));
}

#[test]
fn accepts_the_boundaries() {
    assert_eq!(parse_utc_offset("UTC+14"), Ok(50_400));
    assert_eq!(parse_utc_offset("UTC-14:00"), Ok(-50_400));
    assert_eq!(parse_utc_offset("UTC+1:59"), Ok(7_140));
}

#[test]
fn rejects_malformed_offsets() {
    for bad in [
        "",
        "   ",
        "6",
        "UTC-",
        "UTC+15",
        "UTC+5:60",
        "UTC-abc",
        "UTC-6:5:1",
        "UTC+1:234",
        "Europe/Madrid",
        "UTC6",
    ] {
        assert_eq!(parse_utc_offset(bad), Err(OffsetError), "spec: {bad:?}");
    }
}

#[test]
fn offset_shifts_the_formatted_time() {
    let epoch = 1_790_021_160; // 21-Sep-2026 20:06 UTC
    let offset = parse_utc_offset("UTC-6").unwrap() as i64;
    assert_eq!(format_timestamp(epoch + offset), "21-Sep-2026 14:06");
}

#[test]
fn offset_can_cross_midnight_backwards() {
    let offset = parse_utc_offset("UTC-6").unwrap() as i64;
    assert_eq!(format_timestamp(offset), "31-Dec-1969 18:00");
}

#[test]
fn formats_every_month_abbreviation() {
    // The 1st of each month in 2026 at 16:07 UTC (exercises zero padding)
    let cases: [(i64, &str); 12] = [
        (1_767_283_620, "01-Jan-2026 16:07"),
        (1_769_962_020, "01-Feb-2026 16:07"),
        (1_772_381_220, "01-Mar-2026 16:07"),
        (1_775_059_620, "01-Apr-2026 16:07"),
        (1_777_651_620, "01-May-2026 16:07"),
        (1_780_330_020, "01-Jun-2026 16:07"),
        (1_782_922_020, "01-Jul-2026 16:07"),
        (1_785_600_420, "01-Aug-2026 16:07"),
        (1_788_278_820, "01-Sep-2026 16:07"),
        (1_790_870_820, "01-Oct-2026 16:07"),
        (1_793_549_220, "01-Nov-2026 16:07"),
        (1_796_141_220, "01-Dec-2026 16:07"),
    ];
    for (epoch, expected) in cases {
        assert_eq!(format_timestamp(epoch), expected);
    }
}
