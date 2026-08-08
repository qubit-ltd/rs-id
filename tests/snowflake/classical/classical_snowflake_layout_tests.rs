// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the classic Snowflake bit layout.

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use qubit_id::ClassicalSnowflakeLayout;
use qubit_id::IdGenerationError;

/// Finds the latest whole-second time representable by [`SystemTime`].
///
/// # Returns
///
/// The latest representable time on or after [`UNIX_EPOCH`] whose subsecond
/// component is zero.
fn latest_representable_whole_second() -> SystemTime {
    let mut low = 0_u64;
    let mut high = u64::MAX;
    while low < high {
        let difference = high - low;
        let middle = low + difference / 2 + difference % 2;
        if UNIX_EPOCH
            .checked_add(Duration::from_secs(middle))
            .is_some()
        {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    UNIX_EPOCH
        .checked_add(Duration::from_secs(low))
        .expect("binary search must retain a representable time")
}

#[test]
fn test_snowflake_layout_compose_decode_round_trip() {
    let layout = ClassicalSnowflakeLayout::new(17)
        .expect("node id must fit the classic layout");
    let id = layout
        .compose(123_456, 789)
        .expect("parts must fit the classic layout");
    let parts = ClassicalSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 123_456);
    assert_eq!(parts.node_id(), 17);
    assert_eq!(parts.sequence(), 789);
}

#[test]
fn test_snowflake_layout_getters_return_configuration_and_limits() {
    let layout = ClassicalSnowflakeLayout::new(23)
        .expect("node id must fit the classic layout");

    assert_eq!(layout.node_id(), 23);
    assert_eq!(layout.max_timestamp(), (1_u64 << 41) - 1);
    assert_eq!(layout.max_sequence(), (1_u64 << 12) - 1);
}

#[test]
fn test_snowflake_layout_rejects_out_of_range_node() {
    let error = ClassicalSnowflakeLayout::new(1_u64 << 10)
        .expect_err("node id above 10 bits must fail");

    assert!(matches!(
        error,
        IdGenerationError::NodeOutOfRange {
            node_id: 1_024,
            max: 1_023,
        }
    ));
}

#[test]
fn test_snowflake_layout_rejects_out_of_range_parts() {
    let layout = ClassicalSnowflakeLayout::new(0)
        .expect("node id must fit the classic layout");

    assert!(matches!(
        layout.compose(layout.max_timestamp() + 1, 0),
        Err(IdGenerationError::TimestampOverflow { .. })
    ));
    assert!(matches!(
        layout.compose(0, layout.max_sequence() + 1),
        Err(IdGenerationError::SequenceOverflow { .. })
    ));
}

#[test]
fn test_snowflake_layout_accepts_maximum_parts() {
    let layout = ClassicalSnowflakeLayout::new((1_u64 << 10) - 1)
        .expect("maximum node id must fit the classic layout");
    let id = layout
        .compose(layout.max_timestamp(), layout.max_sequence())
        .expect("maximum parts must fit the classic layout");
    let parts = ClassicalSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), layout.max_timestamp());
    assert_eq!(parts.node_id(), layout.node_id());
    assert_eq!(parts.sequence(), layout.max_sequence());
}

#[test]
fn test_snowflake_layout_calculates_exclusive_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = ClassicalSnowflakeLayout::new(17)
        .expect("node id must fit the classic layout");

    assert_eq!(
        layout
            .expires_at(epoch)
            .expect("classic expiration must be representable"),
        epoch + Duration::from_millis(1_u64 << 41),
    );
}

#[test]
fn test_snowflake_layout_reports_expiration_time_overflow() {
    let origin = latest_representable_whole_second();
    let layout = ClassicalSnowflakeLayout::new(17)
        .expect("node id must fit the classic layout");
    let time_unit = Duration::from_millis(1);

    assert!(matches!(
        layout.expires_at(origin),
        Err(IdGenerationError::ExpirationTimeOverflow {
            origin: actual_origin,
            time_unit: actual_time_unit,
            max_timestamp,
        }) if actual_origin == origin
            && actual_time_unit == time_unit
            && max_timestamp == layout.max_timestamp()
    ));
}
