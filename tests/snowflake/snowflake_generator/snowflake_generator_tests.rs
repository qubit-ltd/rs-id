/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the classic Snowflake generator.

use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    IdError,
    IdGenerator,
    SnowflakeGenerator,
};

#[test]
fn test_snowflake_generator_compose_and_extract_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_epoch(513, epoch).expect("node id should be valid");

    let id = generator.compose(1_234_567, 2_117).expect("parts should be valid");

    assert_eq!(generator.node_id(), 513);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.extract_timestamp(id), 1_234_567);
    assert_eq!(generator.extract_node_id(id), 513);
    assert_eq!(generator.extract_sequence(id), 2_117);
}

#[test]
fn test_snowflake_generator_rejects_invalid_node_and_parts() {
    match SnowflakeGenerator::new(1_024) {
        Err(error) => assert_eq!(
            error,
            IdError::NodeOutOfRange {
                node_id: 1_024,
                max: 1_023,
            }
        ),
        Ok(_) => panic!("invalid node id should be rejected"),
    }

    let generator = SnowflakeGenerator::new(1).expect("node id should be valid");
    assert_eq!(
        generator.compose(generator.max_timestamp() + 1, 0),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_timestamp() + 1,
            max: generator.max_timestamp(),
        })
    );
    assert_eq!(
        generator.compose(0, generator.max_sequence() + 1),
        Err(IdError::SequenceOverflow {
            sequence: generator.max_sequence() + 1,
            max: generator.max_sequence(),
        })
    );
}

#[test]
fn test_snowflake_generator_next_string_uses_numeric_string() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || epoch + Duration::from_millis(77))
        .expect("configuration should be valid");

    let id = generator.next_id().expect("id should generate");
    let next_string = generator
        .next_string()
        .expect("string id should generate after numeric id");

    assert_eq!(generator.extract_timestamp(id), 77);
    assert_eq!(next_string, (id + 1).to_string());
}

#[test]
fn test_snowflake_generator_reports_clock_backwards() {
    let current_millis = Arc::new(AtomicU64::new(10));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        epoch + Duration::from_millis(clock_millis.load(Ordering::SeqCst))
    })
    .expect("configuration should be valid");

    generator.next_id().expect("first id should generate");
    current_millis.store(9, Ordering::SeqCst);

    assert_eq!(
        generator.next_id(),
        Err(IdError::ClockMovedBackwards {
            last_timestamp: 10,
            current_timestamp: 9,
            skew_millis: 1,
            max_skew_millis: 0,
        })
    );
}

#[test]
fn test_snowflake_generator_waits_when_sequence_overflows() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        let call = clock_calls.fetch_add(1, Ordering::SeqCst);
        if call <= 4_097 {
            epoch + Duration::from_millis(10)
        } else {
            epoch + Duration::from_millis(11)
        }
    })
    .expect("configuration should be valid");

    for expected_sequence in 0..=4_095 {
        let id = generator.next_id().expect("id should generate");
        assert_eq!(generator.extract_sequence(id), expected_sequence);
    }
    let wrapped = generator
        .next_id()
        .expect("generator should wait for the next millisecond");

    assert_eq!(generator.extract_timestamp(wrapped), 11);
    assert_eq!(generator.extract_sequence(wrapped), 0);
}

#[test]
fn test_snowflake_generator_reports_timestamp_overflow_from_clock() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || epoch + Duration::from_millis((1_u64 << 41) + 1))
        .expect("configuration should be valid");

    assert_eq!(
        generator.next_id(),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_timestamp() + 2,
            max: generator.max_timestamp(),
        })
    );
}

#[test]
fn test_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || epoch - Duration::from_millis(1))
        .expect("configuration should be valid");

    assert_eq!(generator.next_id(), Err(IdError::TimeBeforeEpoch));
}
