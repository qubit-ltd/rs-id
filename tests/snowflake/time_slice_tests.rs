// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    DEFAULT_MAX_SKEW_MILLIS,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    TimestampPrecision,
};

/// Test generator time-slice state increments sequences and advances
/// timestamps.
#[test]
fn test_time_slice_state_is_observable_through_generated_ids() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || {
            let timestamp = if clock_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                10
            } else {
                11
            };
            epoch + Duration::from_millis(timestamp)
        },
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");

    assert_eq!(QubitSnowflakeLayout::decode(first).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(second).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
}
