// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the `Uuid` domain value.

use std::str::FromStr;

use qubit_id::Uuid;

#[test]
fn test_uuid_round_trips_u128_value() {
    let original = 0x0011_2233_4455_6677_8899_aabb_ccdd_eeff_u128;
    let uuid = Uuid::from(original);

    assert_eq!(uuid.value(), original);
    assert_eq!(u128::from(uuid), original);
}

#[test]
fn test_uuid_displays_as_canonical_lowercase_text() {
    let uuid = Uuid::from(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff_u128);

    assert_eq!(uuid.to_string(), "00112233-4455-6677-8899-aabbccddeeff");
}

#[test]
fn test_uuid_parses_canonical_and_uppercase_text() {
    let expected = Uuid::from(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff_u128);

    assert_eq!(
        Uuid::from_str("00112233-4455-6677-8899-aabbccddeeff")
            .expect("UUID should parse"),
        expected
    );
    assert_eq!(
        Uuid::try_from("00112233-4455-6677-8899-AABBCCDDEEFF")
            .expect("UUID should parse"),
        expected
    );
}

#[test]
fn test_uuid_rejects_invalid_text() {
    assert!(Uuid::from_str("not-a-uuid").is_err());
}
