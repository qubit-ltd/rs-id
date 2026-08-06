// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for canonical UUID v4 string generation.

use std::sync::Arc;

use qubit_id::{IdGenerator, UuidV4StringGenerator};
use uuid::{Uuid, Variant};

/// Asserts that `value` is a canonical hyphenated UUID v4 string.
///
/// # Parameters
///
/// * `value` - UUID string to validate.
fn assert_uuid_v4_string(value: &str) {
    assert_eq!(value.len(), 36);
    assert_eq!(value.as_bytes()[8], b'-');
    assert_eq!(value.as_bytes()[13], b'-');
    assert_eq!(value.as_bytes()[18], b'-');
    assert_eq!(value.as_bytes()[23], b'-');
    let uuid = Uuid::parse_str(value).expect("UUID string should parse");
    assert_eq!(uuid.get_version_num(), 4);
    assert_eq!(uuid.get_variant(), Variant::RFC4122);
}

#[test]
fn test_uuid_v4_string_generator_returns_canonical_string() {
    let generator: Arc<dyn IdGenerator<String>> = Arc::new(UuidV4StringGenerator::new());

    let value = generator.generate().expect("UUID should generate");

    assert_uuid_v4_string(&value);
}

mod inherent_api_tests {
    use super::assert_uuid_v4_string;
    use qubit_id::UuidV4StringGenerator;

    #[test]
    fn test_uuid_v4_string_generator_supports_inherent_generate() {
        let generator = UuidV4StringGenerator::new();
        let value = generator
            .generate()
            .expect("inherent generation should succeed");

        assert_uuid_v4_string(&value);
    }
}
