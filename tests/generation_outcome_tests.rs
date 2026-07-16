// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for non-blocking generation outcomes.

use std::time::Duration;

use qubit_id::GenerationOutcome;

#[test]
fn test_generation_outcome_map_transforms_generated_value() {
    let outcome = GenerationOutcome::Generated(21_u64).map(|value| value * 2);

    assert_eq!(outcome, GenerationOutcome::Generated(42));
}

#[test]
fn test_generation_outcome_map_preserves_retry_after() {
    let duration = Duration::from_millis(25);
    let outcome = GenerationOutcome::<u64>::RetryAfter(duration)
        .map(|value| value.to_string());

    assert_eq!(outcome, GenerationOutcome::RetryAfter(duration));
}
