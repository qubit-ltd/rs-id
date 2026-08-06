// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for non-blocking allocation results.

use std::time::Duration;

use qubit_id::GenerationAttempt;

/// Converts a numeric test identifier to its string representation.
fn stringify_id(id: u64) -> String {
    id.to_string()
}

#[test]
fn test_generation_attempt_maps_generated_values() {
    let attempt = GenerationAttempt::Generated(7_u64).map(stringify_id);
    assert_eq!(attempt, GenerationAttempt::Generated("7".to_owned()));
}

#[test]
fn test_generation_attempt_preserves_retry_delay() {
    let attempt = std::hint::black_box(GenerationAttempt::<u64>::RetryAfter {
        delay: Duration::from_millis(3),
    })
    .map(stringify_id);

    assert_eq!(
        attempt,
        GenerationAttempt::RetryAfter {
            delay: Duration::from_millis(3),
        }
    );

    let retry = GenerationAttempt::RetryAfter {
        delay: Duration::from_millis(5),
    }
    .map(|value: u32| value + 1);
    assert_eq!(
        retry,
        GenerationAttempt::RetryAfter {
            delay: Duration::from_millis(5),
        }
    );
}
