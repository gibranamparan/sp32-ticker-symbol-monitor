use price::{format_price, parse_spot_response, ParseError};

/// The recorded real response from `GET /v2/prices/BTC-USD/spot`
/// (see scripts history: captured 2026-09-19).
#[test]
fn real_fixture_parses() {
    let body = include_str!("fixtures/spot_price.json");
    let value = parse_spot_response(body).expect("recorded fixture must parse");
    assert!(value > 1000.0, "BTC/USD sanity range, got {value}");
}

#[test]
fn exact_recorded_body_matches() {
    // Same shape as the fixture, with the value spelled out so a future
    // re-record of the fixture can't silently weaken this assertion.
    let body = r#"{"data":{"amount":"81280.005","base":"BTC","currency":"USD"}}"#;
    let value = parse_spot_response(body).unwrap();
    assert!((value - 81280.005).abs() < 0.0001);
}

#[test]
fn extra_fields_are_tolerated() {
    let body = r#"{"data":{"amount":"67432.15","base":"BTC","currency":"USD","extra":1},"warnings":[]}"#;
    let value = parse_spot_response(body).unwrap();
    assert!((value - 67432.15).abs() < 0.0001);
}

#[test]
fn whitespace_around_amount_is_tolerated() {
    let value = parse_spot_response(r#"{"data":{"amount":" 67432.15 "}}"#).unwrap();
    assert!((value - 67432.15).abs() < 0.0001);
}

#[test]
fn empty_body_is_rejected() {
    assert_eq!(parse_spot_response(""), Err(ParseError));
}

#[test]
fn invalid_json_is_rejected() {
    assert_eq!(parse_spot_response("not json at all"), Err(ParseError));
}

#[test]
fn missing_data_field_is_rejected() {
    assert_eq!(parse_spot_response(r#"{"error":"boom"}"#), Err(ParseError));
}

#[test]
fn missing_amount_is_rejected() {
    assert_eq!(
        parse_spot_response(r#"{"data":{"base":"BTC","currency":"USD"}}"#),
        Err(ParseError)
    );
}

#[test]
fn non_numeric_amount_is_rejected() {
    assert_eq!(
        parse_spot_response(r#"{"data":{"amount":"abc"}}"#),
        Err(ParseError)
    );
}

#[test]
fn empty_amount_is_rejected() {
    assert_eq!(parse_spot_response(r#"{"data":{"amount":""}}"#), Err(ParseError));
}

#[test]
fn json_number_amount_is_rejected() {
    // The API sends the amount as a string; a bare JSON number is not the
    // documented schema, so it must not slip through.
    assert_eq!(parse_spot_response(r#"{"data":{"amount":67432.15}}"#), Err(ParseError));
}

#[test]
fn nan_amount_is_rejected() {
    assert_eq!(
        parse_spot_response(r#"{"data":{"amount":"NaN"}}"#),
        Err(ParseError)
    );
}

#[test]
fn formats_whole_dollars_with_separators() {
    assert_eq!(format_price(67432.15), "$67,432");
    assert_eq!(format_price(81280.005), "$81,280");
    assert_eq!(format_price(999.4), "$999");
    assert_eq!(format_price(42.0), "$42");
}

#[test]
fn formats_millions_and_billions() {
    assert_eq!(format_price(1_000_000.0), "$1,000,000");
    assert_eq!(format_price(12_345_678.9), "$12,345,679");
}

#[test]
fn parse_and_format_roundtrip() {
    let body = include_str!("fixtures/spot_price.json");
    let value = parse_spot_response(body).unwrap();
    assert_eq!(format_price(value), "$81,280");
}
