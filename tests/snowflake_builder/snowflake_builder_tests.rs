/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for the Java-compatible Qubit snowflake builder.

use qubit_id::{HOST_MAX, HOST_MIN, IdError, IdMode, QubitSnowflakeBuilder, TimestampPrecision};

#[test]
fn test_qubit_snowflake_builder_builds_java_second_sequential_id() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Second, 317)
        .expect("host should be accepted");
    let id = 0b0000000000010010110101101000011111001111011010110100011111000111_u64;

    assert_eq!(builder.build(1_234_567, 2_836_423), Ok(id));
    assert_eq!(builder.extract_mode(id), IdMode::Sequential);
    assert_eq!(builder.extract_timestamp(id), 1_234_567);
    assert_eq!(builder.extract_precision(id), TimestampPrecision::Second);
    assert_eq!(builder.extract_host(id), 317);
    assert_eq!(builder.extract_sequence(id), 2_836_423);
}

#[test]
fn test_qubit_snowflake_builder_builds_java_second_spread_id() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Spread, TimestampPrecision::Second, 317)
        .expect("host should be accepted");
    let id = 0b1111000010110101101001000000000011001111011010110100011111000111_u64;

    assert_eq!(builder.build(1_234_567, 2_836_423), Ok(id));
    assert_eq!(builder.extract_mode(id), IdMode::Spread);
    assert_eq!(builder.extract_timestamp(id), 1_234_567);
    assert_eq!(builder.extract_precision(id), TimestampPrecision::Second);
    assert_eq!(builder.extract_host(id), 317);
    assert_eq!(builder.extract_sequence(id), 2_836_423);
}

#[test]
fn test_qubit_snowflake_builder_builds_java_millisecond_sequential_id() {
    let builder =
        QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Millisecond, 317)
            .expect("host should be accepted");
    let id = 0b0000000000000000000001001011010110100001110100111101100001000101_u64;

    assert_eq!(builder.build(1_234_567, 2_117), Ok(id));
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
fn test_qubit_snowflake_builder_builds_java_millisecond_spread_id() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Spread, TimestampPrecision::Millisecond, 317)
        .expect("host should be accepted");
    let id = 0b1111000010110101101001000000000000000000000100111101100001000101_u64;

    assert_eq!(builder.build(1_234_567, 2_117), Ok(id));
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
fn test_qubit_snowflake_builder_getters_and_default_match_java_defaults() {
    let builder = QubitSnowflakeBuilder::default();

    assert_eq!(builder.mode(), IdMode::Sequential);
    assert_eq!(builder.precision(), TimestampPrecision::Second);
    assert_eq!(builder.host(), 0);
    assert_eq!(builder.max_timestamp(), (1_u64 << 31) - 1);
    assert_eq!(builder.max_sequence(), (1_u64 << 22) - 1);
    assert_eq!(TimestampPrecision::Millisecond.wait_duration_millis(), 1);
    assert_eq!(TimestampPrecision::Second.wait_duration_millis(), 500);
}
