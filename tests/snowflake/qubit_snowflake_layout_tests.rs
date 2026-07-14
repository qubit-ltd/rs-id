// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Qubit snowflake layout.

use qubit_id::{
    HOST_MAX,
    HOST_MIN,
    IdError,
    IdMode,
    QubitSnowflakeLayout,
    TimestampPrecision,
};

/// Tests all mode and precision combinations against the documented bit
/// layout.
#[test]
fn test_compose_all_fixed_header_layouts() {
    let timestamp = 1_234_567_u64;
    let host = 317_u64;

    for mode in [IdMode::Sequential, IdMode::Spread] {
        for precision in
            [TimestampPrecision::Millisecond, TimestampPrecision::Second]
        {
            let layout = QubitSnowflakeLayout::new(mode, precision, host)
                .expect("host should be accepted");
            let sequence = if precision == TimestampPrecision::Millisecond {
                2_117
            } else {
                2_836_423
            };
            let timestamp_bits = precision.timestamp_bits();
            let sequence_bits = precision.sequence_bits();
            let stored_timestamp = if mode == IdMode::Sequential {
                timestamp
            } else {
                timestamp.reverse_bits() >> (u64::BITS as u8 - timestamp_bits)
            };
            let expected = (mode.ordinal() << 63)
                | (precision.ordinal() << 62)
                | (stored_timestamp << (9 + sequence_bits))
                | (host << sequence_bits)
                | sequence;

            assert_eq!(layout.compose(timestamp, sequence), Ok(expected));
        }
    }
}

/// Tests layout validation at every public numeric boundary.
#[test]
fn test_new_and_compose_reject_out_of_range_parts() {
    assert_eq!(HOST_MIN, 0);
    assert_eq!(HOST_MAX, 511);
    assert_eq!(
        QubitSnowflakeLayout::new(
            IdMode::Sequential,
            TimestampPrecision::Second,
            HOST_MAX + 1,
        ),
        Err(IdError::HostOutOfRange {
            host: HOST_MAX + 1,
            max: HOST_MAX,
        })
    );

    let layout = QubitSnowflakeLayout::default();
    assert_eq!(
        layout.compose(layout.max_timestamp() + 1, 0),
        Err(IdError::TimestampOverflow {
            timestamp: layout.max_timestamp() + 1,
            max: layout.max_timestamp(),
        })
    );
    assert_eq!(
        layout.compose(0, layout.max_sequence() + 1),
        Err(IdError::SequenceOverflow {
            sequence: layout.max_sequence() + 1,
            max: layout.max_sequence(),
        })
    );
}

/// Tests layout getters and default values.
#[test]
fn test_default_and_getters_match_qubit_defaults() {
    let layout = QubitSnowflakeLayout::default();

    assert_eq!(layout.mode(), IdMode::Sequential);
    assert_eq!(layout.precision(), TimestampPrecision::Second);
    assert_eq!(layout.host(), 0);
    assert_eq!(layout.max_timestamp(), (1_u64 << 31) - 1);
    assert_eq!(layout.max_sequence(), (1_u64 << 22) - 1);
}

/// Tests that decoding derives its layout from the ID header.
#[test]
fn test_decode_is_configuration_independent() {
    for mode in [IdMode::Sequential, IdMode::Spread] {
        for precision in
            [TimestampPrecision::Millisecond, TimestampPrecision::Second]
        {
            let layout = QubitSnowflakeLayout::new(mode, precision, 511)
                .expect("host should be valid");
            let timestamp = layout.max_timestamp();
            let sequence = layout.max_sequence();
            let id = layout
                .compose(timestamp, sequence)
                .expect("maximum parts should fit");

            let parts = QubitSnowflakeLayout::decode(id);

            assert_eq!(parts.mode(), mode);
            assert_eq!(parts.precision(), precision);
            assert_eq!(parts.timestamp(), timestamp);
            assert_eq!(parts.host(), 511);
            assert_eq!(parts.sequence(), sequence);
        }
    }
}
