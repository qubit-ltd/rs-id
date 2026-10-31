// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Sonyflake bit layout.

use std::time::Duration;
use std::time::UNIX_EPOCH;

use qubit_id::IdGenerationError;
use qubit_id::SonyflakeLayout;

use crate::support::latest_representable_whole_second;

/// Locks default and custom Sonyflake formats using independent numeric
/// vectors.
#[test]
fn test_sonyflake_layout_matches_fixed_encoding_vectors() {
    let cases = [
        (8_u8, 16_u8, 0_u64, 0_u64, 0_u64, 0_u64),
        (8, 16, 456_789, 23, 31, 7_663_649_751_063),
        (8, 16, 549_755_813_887, 65_535, 255, 9_223_372_036_854_775_807),
        (12, 12, 42, 7, 3, 704_655_367),
    ];
    for (bits_sequence, bits_machine, elapsed, machine, sequence, raw) in cases {
        let layout = SonyflakeLayout::new(machine, bits_sequence, bits_machine, Duration::from_millis(10))
            .expect("valid fixed layout");
        assert_eq!(layout.compose_raw(elapsed, sequence).unwrap(), raw);
        assert_eq!(layout.compose(elapsed, sequence).unwrap().value(), raw);
        for parts in [layout.decode_raw(raw), layout.decode(raw.into())] {
            assert_eq!(parts.elapsed_time(), elapsed);
            assert_eq!(parts.machine_id(), machine);
            assert_eq!(parts.sequence(), sequence);
        }
    }
}

#[test]
fn test_sonyflake_layout_compose_decode_round_trip() {
    let layout = SonyflakeLayout::new(23, 8, 16, Duration::from_millis(10)).expect("Sonyflake layout must be valid");
    let id = layout
        .compose(456_789, 31)
        .expect("parts must fit the Sonyflake layout");
    let parts = layout.decode(id);

    assert_eq!(parts.elapsed_time(), 456_789);
    assert_eq!(parts.sequence(), 31);
    assert_eq!(parts.machine_id(), 23);
}

#[test]
fn test_review_decode_extracts_default_and_custom_layout_fields() {
    let default_layout =
        SonyflakeLayout::new(0, 0, 0, Duration::from_millis(10)).expect("default layout must be valid");
    let custom_layout = SonyflakeLayout::new(1, 4, 8, Duration::from_millis(1)).expect("custom layout must be valid");
    let maximum_63_bit_id = (1_u64 << 63) - 1;

    for layout in [default_layout, custom_layout] {
        for raw in [0, maximum_63_bit_id, 1_u64 << 63, u64::MAX] {
            let raw_parts = layout.decode_raw(raw);
            let typed_parts = layout.decode(raw.into());

            assert_eq!(typed_parts, raw_parts);
        }
    }

    let default_high_bit_parts = default_layout.decode_raw(1_u64 << 63);
    assert_eq!(default_high_bit_parts.elapsed_time(), 1_u64 << 39);

    let custom_high_bit_parts = custom_layout.decode_raw(1_u64 << 63);
    assert_eq!(custom_high_bit_parts.elapsed_time(), 1_u64 << 51);
    assert!(custom_high_bit_parts.elapsed_time() > custom_layout.max_elapsed_time());
}

#[test]
fn test_sonyflake_layout_getters_return_configuration_and_limits() {
    let time_unit = Duration::from_millis(10);
    let layout = SonyflakeLayout::new(23, 8, 16, time_unit).expect("Sonyflake layout must be valid");

    assert_eq!(layout.bits_time(), 39);
    assert_eq!(layout.bits_sequence(), 8);
    assert_eq!(layout.bits_machine(), 16);
    assert_eq!(layout.time_unit(), time_unit);
    assert_eq!(layout.machine_id(), 23);
    assert_eq!(layout.max_elapsed_time(), (1_u64 << 39) - 1);
    assert_eq!(layout.max_sequence(), (1_u64 << 8) - 1);
    assert_eq!(layout.max_machine_id(), (1_u64 << 16) - 1);
}

#[test]
fn test_sonyflake_layout_zero_bit_widths_select_defaults() {
    let layout =
        SonyflakeLayout::new(23, 0, 0, Duration::from_millis(10)).expect("zero widths must select Sonyflake defaults");

    assert_eq!(layout.bits_time(), 39);
    assert_eq!(layout.bits_sequence(), 8);
    assert_eq!(layout.bits_machine(), 16);
}

#[test]
fn test_sonyflake_layout_rejects_invalid_bit_widths() {
    let time_unit = Duration::from_millis(10);

    assert!(matches!(
        SonyflakeLayout::new(0, 31, 16, time_unit),
        Err(IdGenerationError::InvalidBitLength {
            name: "sequence",
            bits: 31,
            ..
        })
    ));
    assert!(matches!(
        SonyflakeLayout::new(0, 8, 31, time_unit),
        Err(IdGenerationError::InvalidBitLength {
            name: "machine",
            bits: 31,
            ..
        })
    ));
    assert!(matches!(
        SonyflakeLayout::new(0, 16, 16, time_unit),
        Err(IdGenerationError::InvalidBitLength {
            name: "time",
            bits: 31,
            ..
        })
    ));
}

#[test]
fn test_sonyflake_layout_rejects_invalid_time_unit_and_machine() {
    assert!(matches!(
        SonyflakeLayout::new(0, 8, 16, Duration::from_nanos(999_999)),
        Err(IdGenerationError::InvalidTimeUnit {
            nanos: 999_999,
            min_nanos: 1_000_000,
        })
    ));
    assert!(matches!(
        SonyflakeLayout::new(1_u64 << 16, 8, 16, Duration::from_millis(10),),
        Err(IdGenerationError::MachineIdOutOfRange {
            machine_id: 65_536,
            max: 65_535,
        })
    ));
}

#[test]
fn test_sonyflake_layout_rejects_out_of_range_parts() {
    let layout = SonyflakeLayout::new(23, 8, 16, Duration::from_millis(10)).expect("Sonyflake layout must be valid");

    assert!(matches!(
        layout.compose(layout.max_elapsed_time() + 1, 0),
        Err(IdGenerationError::TimestampOverflow { .. })
    ));
    assert!(matches!(
        layout.compose(0, layout.max_sequence() + 1),
        Err(IdGenerationError::SequenceOverflow { .. })
    ));
}

#[test]
fn test_sonyflake_layout_accepts_maximum_parts() {
    let layout = SonyflakeLayout::new((1_u64 << 16) - 1, 8, 16, Duration::from_millis(10))
        .expect("maximum machine id must fit the Sonyflake layout");
    let id = layout
        .compose(layout.max_elapsed_time(), layout.max_sequence())
        .expect("maximum parts must fit the Sonyflake layout");
    let parts = layout.decode(id);

    assert_eq!(parts.elapsed_time(), layout.max_elapsed_time());
    assert_eq!(parts.sequence(), layout.max_sequence());
    assert_eq!(parts.machine_id(), layout.machine_id());
}

#[test]
fn test_sonyflake_layout_calculates_exclusive_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time_unit = Duration::from_millis(10);
    let layout = SonyflakeLayout::new(23, 8, 16, time_unit).expect("Sonyflake layout must be valid");

    assert_eq!(
        layout
            .expires_at(epoch)
            .expect("Sonyflake expiration must be representable"),
        epoch + Duration::from_millis((1_u64 << 39) * 10),
    );
}

#[test]
fn test_sonyflake_layout_reports_expiration_time_overflow() {
    let origin = latest_representable_whole_second();
    let time_unit = Duration::from_millis(10);
    let layout = SonyflakeLayout::new(23, 8, 16, time_unit).expect("Sonyflake layout must be valid");

    assert!(matches!(
        layout.expires_at(origin),
        Err(IdGenerationError::ExpirationTimeOverflow {
            origin: actual_origin,
            time_unit: actual_time_unit,
            max_timestamp,
        }) if actual_origin == origin
            && actual_time_unit == time_unit
            && max_timestamp == layout.max_elapsed_time()
    ));
}

#[test]
fn test_sonyflake_layout_reports_lifetime_multiplication_overflow() {
    let layout = SonyflakeLayout::new(1, 8, 16, Duration::MAX).expect("field widths and machine ID should be valid");

    assert!(matches!(
        layout.expires_at(UNIX_EPOCH),
        Err(IdGenerationError::ExpirationTimeOverflow { .. })
    ));
}

#[test]
fn test_sonyflake_layout_reports_lifetime_duration_overflow() {
    let time_unit = Duration::from_secs(u64::MAX);
    let layout = SonyflakeLayout::new(1, 15, 16, time_unit).expect("field widths and machine ID should be valid");

    assert!(matches!(
        layout.expires_at(UNIX_EPOCH),
        Err(IdGenerationError::ExpirationTimeOverflow {
            origin,
            time_unit: actual_time_unit,
            max_timestamp,
        }) if origin == UNIX_EPOCH
            && actual_time_unit == time_unit
            && max_timestamp == layout.max_elapsed_time()
    ));
}
