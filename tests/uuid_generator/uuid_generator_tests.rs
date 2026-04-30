/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for fast UUID generation and formatting.

use qubit_id::{IdGenerator, UuidGenerator, fast_simple_uuid, fast_uuid};

#[test]
fn test_uuid_generator_formats_u128_as_canonical_uuid() {
    let value = 0x12345678_9abc_def0_1234_56789abcdef0_u128;

    assert_eq!(
        UuidGenerator::format_uuid(value),
        "12345678-9abc-def0-1234-56789abcdef0"
    );
    assert_eq!(
        UuidGenerator::format_simple_uuid(value),
        "123456789abcdef0123456789abcdef0"
    );
}

#[test]
fn test_uuid_generator_next_string_uses_uuid_format() {
    let generator = UuidGenerator::new();

    let id = generator.next_id().expect("uuid should generate");
    let uuid = UuidGenerator::format_uuid(id);
    let next = generator
        .next_string()
        .expect("uuid string should generate");

    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid.chars().filter(|ch| *ch == '-').count(), 4);
    assert_eq!(next.len(), 36);
    assert_eq!(next.as_bytes()[8], b'-');
    assert_eq!(next.as_bytes()[13], b'-');
    assert_eq!(next.as_bytes()[18], b'-');
    assert_eq!(next.as_bytes()[23], b'-');
}

#[test]
fn test_fast_uuid_helpers_return_lowercase_hex_shapes() {
    let uuid = fast_uuid().expect("uuid should generate");
    let simple = fast_simple_uuid().expect("simple uuid should generate");

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
