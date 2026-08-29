// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for decoded Qubit snowflake parts.

use qubit_id::IdMode;
use qubit_id::SnowflakeLayout;
use qubit_id::TimestampPrecision;

/// Tests all decoded field accessors and value semantics.
#[test]
fn test_accessors_return_decoded_fields() {
    let layout =
        SnowflakeLayout::new(IdMode::Spread, TimestampPrecision::Millisecond, 37).expect("host should be valid");
    let id = layout.compose(12_345, 67).expect("timestamp and sequence should fit");

    let parts = SnowflakeLayout::decode(id);
    let copied = parts;

    assert_eq!(parts, copied);
    assert_eq!(parts.mode(), IdMode::Spread);
    assert_eq!(parts.precision(), TimestampPrecision::Millisecond);
    assert_eq!(parts.timestamp(), 12_345);
    assert_eq!(parts.host(), 37);
    assert_eq!(parts.sequence(), 67);
}
