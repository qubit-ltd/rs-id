/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for Mica UUID-like generation and formatting.

use qubit_id::{
    IdGenerator,
    MicaUuidLikeGenerator,
    fast_simple_uuid_like,
    fast_uuid_like,
};

#[test]
fn test_mica_uuid_like_generator_formats_u128_as_canonical_uuid_like_text() {
    let value = 0x12345678_9abc_def0_1234_56789abcdef0_u128;

    assert_eq!(
        MicaUuidLikeGenerator::format_uuid_like(value),
        "12345678-9abc-def0-1234-56789abcdef0"
    );
    assert_eq!(
        MicaUuidLikeGenerator::format_simple_uuid_like(value),
        "123456789abcdef0123456789abcdef0"
    );
}

#[test]
fn test_mica_uuid_like_generator_next_string_uses_uuid_like_format() {
    let generator = MicaUuidLikeGenerator::new();

    let id = generator.next_id().expect("uuid-like id should generate");
    let uuid = MicaUuidLikeGenerator::format_uuid_like(id);
    let next = generator
        .next_string()
        .expect("uuid-like string should generate");

    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid.chars().filter(|ch| *ch == '-').count(), 4);
    assert_eq!(next.len(), 36);
    assert_eq!(next.as_bytes()[8], b'-');
    assert_eq!(next.as_bytes()[13], b'-');
    assert_eq!(next.as_bytes()[18], b'-');
    assert_eq!(next.as_bytes()[23], b'-');
}

#[test]
fn test_fast_uuid_like_helpers_return_lowercase_hex_shapes() {
    let uuid = fast_uuid_like().expect("uuid-like id should generate");
    let simple = fast_simple_uuid_like().expect("simple uuid-like id should generate");

    assert_eq!(uuid.len(), 36);
    assert!(
        uuid.chars()
            .all(|ch| ch == '-' || ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    );
    assert_eq!(simple.len(), 32);
    assert!(
        simple
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    );
}
