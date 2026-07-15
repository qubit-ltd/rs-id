// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
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
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::builder(3)
        .mode(IdMode::Sequential)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .max_skew_millis(DEFAULT_MAX_SKEW_MILLIS)
        .clock(move || epoch + Duration::from_millis(10))
        .build()
        .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");

    assert_eq!(QubitSnowflakeLayout::decode(first).timestamp(), 10);
    assert_eq!(QubitSnowflakeLayout::decode(second).timestamp(), 10);
    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
}
