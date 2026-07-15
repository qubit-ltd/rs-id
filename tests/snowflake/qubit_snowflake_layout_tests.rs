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

const PROPERTY_CASES: usize = 10_000;
const BOUNDARY_IDS: [u64; 8] = [
    0,
    1,
    (1_u64 << 31) - 1,
    1_u64 << 31,
    (1_u64 << 62) - 1,
    1_u64 << 62,
    1_u64 << 63,
    u64::MAX,
];

/// Produces a deterministic pseudo-random value with the SplitMix64 mixer.
fn next_property_value(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

/// Asserts that decoding and recomposing an arbitrary bit pattern is lossless.
fn assert_id_round_trip(id: u64) {
    let parts = QubitSnowflakeLayout::decode(id);
    let layout = QubitSnowflakeLayout::new(
        parts.mode(),
        parts.precision(),
        parts.host(),
    )
    .expect("a decoded host must fit its field");

    assert_eq!(
        layout.compose(parts.timestamp(), parts.sequence()),
        Ok(id),
        "round trip failed for ID {id:#018x}",
    );
}

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

/// Checks that arbitrary 64-bit patterns survive decode-and-compose round
/// trips.
#[test]
fn test_arbitrary_id_decode_compose_round_trip() {
    for id in BOUNDARY_IDS {
        assert_id_round_trip(id);
    }

    let mut state = 0xA076_1D64_78BD_642F;
    for _ in 0..PROPERTY_CASES {
        assert_id_round_trip(next_property_value(&mut state));
    }
}

/// Checks that legal parts survive compose-and-decode round trips for every
/// layout.
#[test]
fn test_legal_parts_compose_decode_round_trip() {
    let mut state = 0xE703_7ED1_A0B4_28DB;

    for mode in [IdMode::Sequential, IdMode::Spread] {
        for precision in
            [TimestampPrecision::Millisecond, TimestampPrecision::Second]
        {
            for _ in 0..PROPERTY_CASES {
                let host = next_property_value(&mut state) & HOST_MAX;
                let layout = QubitSnowflakeLayout::new(mode, precision, host)
                    .expect("a masked host must be valid");
                let timestamp =
                    next_property_value(&mut state) & layout.max_timestamp();
                let sequence =
                    next_property_value(&mut state) & layout.max_sequence();
                let id = layout
                    .compose(timestamp, sequence)
                    .expect("masked parts must fit their fields");

                let parts = QubitSnowflakeLayout::decode(id);

                assert_eq!(parts.mode(), mode);
                assert_eq!(parts.precision(), precision);
                assert_eq!(parts.timestamp(), timestamp);
                assert_eq!(parts.host(), host);
                assert_eq!(parts.sequence(), sequence);
            }
        }
    }
}
