/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the Qubit snowflake generator.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::thread;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    DEFAULT_MAX_SKEW_MILLIS,
    IdError,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    TimestampPrecision,
};

#[test]
fn test_qubit_snowflake_generator_generate_at_matches_builder_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        7,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch + Duration::from_millis(123),
    )
    .expect("configuration should be valid");

    let id = generator
        .generate_at(epoch + Duration::from_millis(45), 9)
        .expect("timestamp and sequence should be valid");

    assert_eq!(generator.builder().extract_timestamp(id), 45);
    assert_eq!(generator.builder().extract_sequence(id), 9);
    assert_eq!(generator.builder().extract_host(id), 7);
    assert_eq!(generator.epoch(), epoch);
}

#[test]
fn test_qubit_snowflake_generator_next_id_increments_sequence_in_same_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");

    assert_eq!(generator.builder().extract_timestamp(first), 10);
    assert_eq!(generator.builder().extract_timestamp(second), 10);
    assert_eq!(generator.builder().extract_sequence(first), 0);
    assert_eq!(generator.builder().extract_sequence(second), 1);
    assert_eq!(
        generator.next_string().expect("string id should generate"),
        second.wrapping_add(1).to_string()
    );
}

#[test]
fn test_qubit_snowflake_generator_reports_large_clock_backwards() {
    let current_millis = Arc::new(AtomicU64::new(10));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        0,
        move || epoch + Duration::from_millis(clock_millis.load(Ordering::SeqCst)),
    )
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
fn test_qubit_snowflake_generator_waits_for_small_clock_backwards() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        2,
        move || {
            let call = clock_calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => epoch + Duration::from_millis(10),
                1 => epoch + Duration::from_millis(9),
                _ => epoch + Duration::from_millis(10),
            }
        },
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator
        .next_id()
        .expect("small clock skew should wait and retry");

    assert_eq!(generator.builder().extract_sequence(first), 0);
    assert_eq!(generator.builder().extract_sequence(second), 1);
}

#[test]
fn test_qubit_snowflake_generator_waits_when_sequence_overflows() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || {
            let call = clock_calls.fetch_add(1, Ordering::SeqCst);
            if call <= 4_097 {
                epoch + Duration::from_millis(10)
            } else {
                epoch + Duration::from_millis(11)
            }
        },
    )
    .expect("configuration should be valid");

    for expected_sequence in 0..=4_095 {
        let id = generator.next_id().expect("id should generate");
        assert_eq!(generator.builder().extract_sequence(id), expected_sequence);
    }
    let wrapped = generator
        .next_id()
        .expect("generator should wait for the next timestamp");

    assert_eq!(generator.builder().extract_timestamp(wrapped), 11);
    assert_eq!(generator.builder().extract_sequence(wrapped), 0);
}

#[test]
fn test_qubit_snowflake_generator_reports_timestamp_overflow_from_time() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch,
    )
    .expect("configuration should be valid");
    let timestamp = generator.builder().max_timestamp() + 1;

    assert_eq!(
        generator.generate_at(epoch + Duration::from_millis(timestamp), 0),
        Err(IdError::TimestampOverflow {
            timestamp,
            max: generator.builder().max_timestamp(),
        })
    );
}

#[test]
fn test_qubit_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch,
    )
    .expect("configuration should be valid");

    assert_eq!(
        generator.generate_at(epoch - Duration::from_millis(1), 0),
        Err(IdError::TimeBeforeEpoch)
    );
}

#[test]
fn test_qubit_snowflake_generator_rejects_invalid_host_from_clock_constructor() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);

    assert!(matches!(
        QubitSnowflakeGenerator::with_clock(
            IdMode::Sequential,
            TimestampPrecision::Millisecond,
            512,
            epoch,
            DEFAULT_MAX_SKEW_MILLIS,
            move || epoch,
        ),
        Err(IdError::HostOutOfRange {
            host: 512,
            max: 511
        })
    ));
}

#[test]
fn test_qubit_snowflake_generator_is_thread_safe() {
    let generator = Arc::new(QubitSnowflakeGenerator::new(11).expect("host should be valid"));
    let mut handles = Vec::new();

    for _ in 0..4 {
        let generator = Arc::clone(&generator);
        handles.push(thread::spawn(move || {
            let mut ids = Vec::new();
            for _ in 0..128 {
                ids.push(generator.next_id().expect("id should generate"));
            }
            ids
        }));
    }

    let mut ids = HashSet::new();
    for handle in handles {
        for id in handle.join().expect("thread should finish") {
            assert!(ids.insert(id), "duplicate id generated: {id}");
        }
    }
}
