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

#[cfg(feature = "serde")]
mod serde_tests {
    use qubit_id::Id;
    use serde_test::{
        Compact,
        Configure,
        Token,
        assert_de_tokens_error,
        assert_tokens,
    };

    #[test]
    fn test_id_json_uses_decimal_string() {
        assert_eq!(
            serde_json::to_string(&Id::from(42)).expect("serialize ID"),
            "\"42\""
        );
        assert_eq!(
            serde_json::from_str::<Id>("\"42\"").expect("deserialize ID"),
            Id::from(42)
        );
        assert_eq!(
            serde_json::from_str::<Id>("\"18446744073709551615\"")
                .expect("deserialize max ID"),
            Id::from(u64::MAX)
        );
    }

    #[test]
    fn test_id_json_rejects_non_string_values() {
        for input in ["42", "-1", "1.5", "true", "null", "[42]"] {
            assert!(
                serde_json::from_str::<Id>(input).is_err(),
                "accepted {input}"
            );
        }
        assert!(
            serde_json::from_str::<Id>("\"18446744073709551616\"").is_err()
        );
    }

    #[test]
    fn test_id_binary_tokens_use_u64() {
        assert_tokens(&Id::from(u64::MAX).compact(), &[Token::U64(u64::MAX)]);
        assert_de_tokens_error::<Compact<Id>>(
            &[Token::String("42")],
            "invalid type: string \"42\", expected u64",
        );
    }
}
