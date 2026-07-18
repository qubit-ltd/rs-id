// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for canonical UUID v4 string generation.

use std::sync::Arc;

use qubit_id::{
    AsyncIdGenerator,
    IdGenerator,
    UuidV4StringGenerator,
};
use uuid::{
    Uuid,
    Variant,
};

mod concrete_async_tests {
    use qubit_id::UuidV4StringGenerator;

    /// Tests the allocation-free inherent asynchronous API.
    #[tokio::test]
    async fn test_uuid_v4_string_generator_supports_concrete_async_call() {
        let generator = UuidV4StringGenerator::new();

        let value = generator
            .generate_async()
            .await
            .expect("UUID should generate");

        super::assert_uuid_v4_string(&value);
    }
}

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
    let generator: Arc<dyn IdGenerator<String>> =
        Arc::new(UuidV4StringGenerator::new());

    let value = generator.generate().expect("UUID should generate");

    assert_uuid_v4_string(&value);
}

#[tokio::test]
async fn test_uuid_v4_string_generator_supports_async_trait_object() {
    let generator: Arc<dyn AsyncIdGenerator<String>> =
        Arc::new(UuidV4StringGenerator::new());

    let value = generator
        .generate_async()
        .await
        .expect("UUID should generate");

    assert_uuid_v4_string(&value);
}
