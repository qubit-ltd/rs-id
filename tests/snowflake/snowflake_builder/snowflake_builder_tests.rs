/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the Qubit snowflake builder.

use qubit_id::{HOST_MAX, HOST_MIN, IdError, IdMode, QubitSnowflakeBuilder, TimestampPrecision};

#[test]
fn test_qubit_snowflake_builder_builds_second_sequential_id_with_fixed_header() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Second, 317)
        .expect("host should be accepted");
    let id = (TimestampPrecision::Second.ordinal() << 62)
        | (1_234_567_u64 << 31)
        | (317_u64 << 22)
        | 2_836_423_u64;

    assert_eq!(builder.build(1_234_567, 2_836_423), Ok(id));
    assert_eq!((id >> 63) & 1, 0);
    assert_eq!((id >> 62) & 1, 1);
    assert_eq!(builder.extract_mode(id), IdMode::Sequential);
    assert_eq!(builder.extract_timestamp(id), 1_234_567);
    assert_eq!(builder.extract_precision(id), TimestampPrecision::Second);
    assert_eq!(builder.extract_host(id), 317);
    assert_eq!(builder.extract_sequence(id), 2_836_423);
}

#[test]
fn test_qubit_snowflake_builder_builds_second_spread_id_with_fixed_header() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Spread, TimestampPrecision::Second, 317)
        .expect("host should be accepted");
    let stored_timestamp = 1_234_567_u64.reverse_bits() >> (u64::BITS as u8 - 31);
    let id = (IdMode::Spread.ordinal() << 63)
        | (TimestampPrecision::Second.ordinal() << 62)
        | (stored_timestamp << 31)
        | (317_u64 << 22)
        | 2_836_423_u64;

    assert_eq!(builder.build(1_234_567, 2_836_423), Ok(id));
    assert_eq!((id >> 63) & 1, 1);
    assert_eq!((id >> 62) & 1, 1);
    assert_eq!(builder.extract_mode(id), IdMode::Spread);
    assert_eq!(builder.extract_timestamp(id), 1_234_567);
    assert_eq!(builder.extract_precision(id), TimestampPrecision::Second);
    assert_eq!(builder.extract_host(id), 317);
    assert_eq!(builder.extract_sequence(id), 2_836_423);
}

#[test]
fn test_qubit_snowflake_builder_builds_millisecond_sequential_id_with_fixed_header() {
    let builder =
        QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Millisecond, 317)
            .expect("host should be accepted");
    let id = (1_234_567_u64 << 21) | (317_u64 << 12) | 2_117_u64;

    assert_eq!(builder.build(1_234_567, 2_117), Ok(id));
    assert_eq!((id >> 63) & 1, 0);
    assert_eq!((id >> 62) & 1, 0);
    assert_eq!(builder.extract_mode(id), IdMode::Sequential);
    assert_eq!(builder.extract_timestamp(id), 1_234_567);
    assert_eq!(
        builder.extract_precision(id),
        TimestampPrecision::Millisecond
    );
    assert_eq!(builder.extract_host(id), 317);
    assert_eq!(builder.extract_sequence(id), 2_117);
}

#[test]
fn test_qubit_snowflake_builder_builds_millisecond_spread_id_with_fixed_header() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Spread, TimestampPrecision::Millisecond, 317)
        .expect("host should be accepted");
    let stored_timestamp = 1_234_567_u64.reverse_bits() >> (u64::BITS as u8 - 41);
    let id =
        (IdMode::Spread.ordinal() << 63) | (stored_timestamp << 21) | (317_u64 << 12) | 2_117_u64;

    assert_eq!(builder.build(1_234_567, 2_117), Ok(id));
    assert_eq!((id >> 63) & 1, 1);
    assert_eq!((id >> 62) & 1, 0);
    assert_eq!(builder.extract_mode(id), IdMode::Spread);
    assert_eq!(builder.extract_timestamp(id), 1_234_567);
    assert_eq!(
        builder.extract_precision(id),
        TimestampPrecision::Millisecond
    );
    assert_eq!(builder.extract_host(id), 317);
    assert_eq!(builder.extract_sequence(id), 2_117);
}

#[test]
fn test_qubit_snowflake_builder_rejects_invalid_host_and_parts() {
    assert_eq!(HOST_MIN, 0);
    assert_eq!(HOST_MAX, 511);
    assert_eq!(
        QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Second, HOST_MAX + 1),
        Err(IdError::HostOutOfRange {
            host: HOST_MAX + 1,
            max: HOST_MAX,
        })
    );

    let builder = QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Second, 1)
        .expect("host should be accepted");

    assert_eq!(
        builder.build(builder.max_timestamp() + 1, 0),
        Err(IdError::TimestampOverflow {
            timestamp: builder.max_timestamp() + 1,
            max: builder.max_timestamp(),
        })
    );
    assert_eq!(
        builder.build(0, builder.max_sequence() + 1),
        Err(IdError::SequenceOverflow {
            sequence: builder.max_sequence() + 1,
            max: builder.max_sequence(),
        })
    );
}

#[test]
fn test_qubit_snowflake_builder_getters_and_default_match_qubit_defaults() {
    let builder = QubitSnowflakeBuilder::default();

    assert_eq!(builder.mode(), IdMode::Sequential);
    assert_eq!(builder.precision(), TimestampPrecision::Second);
    assert_eq!(builder.host(), 0);
    assert_eq!(builder.max_timestamp(), (1_u64 << 31) - 1);
    assert_eq!(builder.max_sequence(), (1_u64 << 22) - 1);
    assert_eq!(TimestampPrecision::Millisecond.wait_duration_millis(), 1);
    assert_eq!(TimestampPrecision::Second.wait_duration_millis(), 500);
}
