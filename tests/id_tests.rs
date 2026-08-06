// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the `Id` domain value.

use std::str::FromStr;

use qubit_id::Id;

#[test]
fn test_id_round_trips_u64_value() {
    let original = u64::MAX;
    let id = Id::from(original);

    assert_eq!(id.value(), original);
    assert_eq!(u64::from(id), original);
}

#[test]
fn test_id_displays_as_unsigned_decimal() {
    assert_eq!(Id::from(42).to_string(), "42");
}

#[test]
fn test_id_parses_decimal_text() {
    assert_eq!(Id::from_str("42").expect("ID should parse"), Id::from(42));
    assert_eq!(Id::try_from("7").expect("ID should parse"), Id::from(7));
}

#[test]
fn test_id_rejects_invalid_text() {
    assert!(Id::from_str("").is_err());
    assert!(Id::from_str("-1").is_err());
    assert!(Id::from_str("18446744073709551616").is_err());
}
