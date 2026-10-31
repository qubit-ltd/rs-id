// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the classic Snowflake bit layout.

use std::time::Duration;
use std::time::UNIX_EPOCH;

use qubit_id::ClassicalSnowflakeLayout;
use qubit_id::IdGenerationError;

use crate::support::latest_representable_whole_second;

/// Locks the classic persisted bit format using independent numeric vectors.
#[test]
fn test_classical_layout_matches_fixed_encoding_vectors() {
    let cases = [
        (0_u64, 0_u64, 0_u64, 0_u64),
        (123_456, 17, 789, 517_812_065_045),
        (2_199_023_255_551, 1_023, 4_095, 9_223_372_036_854_775_807),
    ];
    for (timestamp, node, sequence, raw) in cases {
        let layout = ClassicalSnowflakeLayout::new(node).expect("valid fixed node");
        assert_eq!(layout.compose_raw(timestamp, sequence).unwrap(), raw);
        assert_eq!(layout.compose(timestamp, sequence).unwrap().value(), raw);
        for parts in [
            ClassicalSnowflakeLayout::decode_raw(raw),
            ClassicalSnowflakeLayout::decode(raw.into()),
        ] {
            assert_eq!(parts.timestamp(), timestamp);
            assert_eq!(parts.node_id(), node);
            assert_eq!(parts.sequence(), sequence);
        }
    }
}

#[test]
fn test_snowflake_layout_compose_decode_round_trip() {
    let layout = ClassicalSnowflakeLayout::new(17).expect("node id must fit the classic layout");
    let id = layout.compose(123_456, 789).expect("parts must fit the classic layout");
    let parts = ClassicalSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 123_456);
    assert_eq!(parts.node_id(), 17);
    assert_eq!(parts.sequence(), 789);
}

#[test]
fn test_review_decode_accepts_only_the_63_bit_layout_as_round_trip_input() {
    let layout = ClassicalSnowflakeLayout::new(0).expect("zero node ID must be valid");
    let maximum_63_bit_id = (1_u64 << 63) - 1;

    for raw in [0, maximum_63_bit_id] {
        let raw_parts = ClassicalSnowflakeLayout::decode_raw(raw);
        let typed_parts = ClassicalSnowflakeLayout::decode(raw.into());

        assert_eq!(typed_parts, raw_parts);
        assert!(layout.compose(raw_parts.timestamp(), raw_parts.sequence()).is_ok());
    }

    let high_bit_parts = ClassicalSnowflakeLayout::decode_raw(1_u64 << 63);
    let typed_high_bit_parts = ClassicalSnowflakeLayout::decode((1_u64 << 63).into());
    assert_eq!(typed_high_bit_parts, high_bit_parts);
    assert_eq!(high_bit_parts.timestamp(), 1_u64 << 41);
    assert!(matches!(
        layout.compose(high_bit_parts.timestamp(), high_bit_parts.sequence()),
        Err(IdGenerationError::TimestampOverflow { .. })
    ));

    let all_bits_parts = ClassicalSnowflakeLayout::decode_raw(u64::MAX);
    let typed_all_bits_parts = ClassicalSnowflakeLayout::decode(u64::MAX.into());
    assert_eq!(typed_all_bits_parts, all_bits_parts);
    assert_eq!(all_bits_parts.timestamp(), (1_u64 << 41) - 1 + (1_u64 << 41));
    assert!(matches!(
        layout.compose(all_bits_parts.timestamp(), all_bits_parts.sequence()),
        Err(IdGenerationError::TimestampOverflow { .. })
    ));
}

#[test]
fn test_snowflake_layout_getters_return_configuration_and_limits() {
    let layout = ClassicalSnowflakeLayout::new(23).expect("node id must fit the classic layout");

    assert_eq!(layout.node_id(), 23);
    assert_eq!(layout.max_timestamp(), (1_u64 << 41) - 1);
    assert_eq!(layout.max_sequence(), (1_u64 << 12) - 1);
}

#[test]
fn test_snowflake_layout_rejects_out_of_range_node() {
    let error = ClassicalSnowflakeLayout::new(1_u64 << 10).expect_err("node id above 10 bits must fail");

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
    let layout = ClassicalSnowflakeLayout::new(0).expect("node id must fit the classic layout");

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
    let layout = ClassicalSnowflakeLayout::new((1_u64 << 10) - 1).expect("maximum node id must fit the classic layout");
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
    let layout = ClassicalSnowflakeLayout::new(17).expect("node id must fit the classic layout");

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
    let layout = ClassicalSnowflakeLayout::new(17).expect("node id must fit the classic layout");
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
