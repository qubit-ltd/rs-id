/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_id::{
    HOST_MAX,
    IdError,
    IdMode,
    QubitSnowflakeBuilder,
    TimestampPrecision,
};

/// Test builder field extraction and validation through the public API.
#[test]
fn test_qubit_snowflake_builder_extracts_encoded_fields() {
    let builder = QubitSnowflakeBuilder::new(IdMode::Spread, TimestampPrecision::Millisecond, 37)
        .expect("host should be valid");
    let id = builder
        .build(12_345, 67)
        .expect("timestamp and sequence should fit");

    assert_eq!(builder.mode(), IdMode::Spread);
    assert_eq!(builder.precision(), TimestampPrecision::Millisecond);
    assert_eq!(builder.host(), 37);
    assert_eq!(builder.extract_mode(id), IdMode::Spread);
    assert_eq!(
        builder.extract_precision(id),
        TimestampPrecision::Millisecond
    );
    assert_eq!(builder.extract_timestamp(id), 12_345);
    assert_eq!(builder.extract_host(id), 37);
    assert_eq!(builder.extract_sequence(id), 67);
}

/// Test builder rejects values that exceed the Qubit layout limits.
#[test]
fn test_qubit_snowflake_builder_rejects_out_of_range_parts() {
    assert_eq!(
        QubitSnowflakeBuilder::new(IdMode::Sequential, TimestampPrecision::Second, HOST_MAX + 1),
        Err(IdError::HostOutOfRange {
            host: HOST_MAX + 1,
            max: HOST_MAX,
        }),
    );

    let builder = QubitSnowflakeBuilder::default();
    assert!(matches!(
        builder.build(builder.max_timestamp() + 1, 0),
        Err(IdError::TimestampOverflow { .. }),
    ));
    assert!(matches!(
        builder.build(0, builder.max_sequence() + 1),
        Err(IdError::SequenceOverflow { .. }),
    ));
}
