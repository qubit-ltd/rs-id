// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the synchronous Qubit Snowflake generator.

use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_id::{
    DEFAULT_MAX_CLOCK_SKEW,
    IdError,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    RestartPolicy,
    TimestampPrecision,
};

use qubit_clock::TimeError;

use crate::support::{
    FailingTimer,
    ManualTime,
};

/// Builds a deterministic Qubit generator and its shared manual timeline.
///
/// # Arguments
///
/// * `precision` - Timestamp precision used by the generated IDs.
/// * `host` - Host identifier encoded by the layout.
/// * `epoch` - Timestamp origin.
/// * `now` - Initial wall time.
/// * `max_clock_skew` - Largest tolerated wall-clock rollback.
///
/// # Returns
///
/// The configured generator and time controller.
fn build_generator(
    precision: TimestampPrecision,
    host: u64,
    epoch: SystemTime,
    now: SystemTime,
    max_clock_skew: Duration,
) -> (QubitSnowflakeGenerator, ManualTime) {
    let time = ManualTime::new(now);
    let generator = QubitSnowflakeGenerator::builder(host)
        .precision(precision)
        .epoch(epoch)
        .max_clock_skew(max_clock_skew)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    (generator, time)
}

#[test]
fn test_qubit_snowflake_generator_new_uses_defaults() {
    let generator = QubitSnowflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().host(), 17);
    assert_eq!(generator.max_clock_skew(), DEFAULT_MAX_CLOCK_SKEW);
}

#[test]
fn test_generate_at_matches_layout_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        7,
        epoch,
        epoch + Duration::from_millis(123),
        DEFAULT_MAX_CLOCK_SKEW,
    );

    let id = generator
        .generate_at(epoch + Duration::from_millis(45), 9)
        .expect("timestamp and sequence should be valid");
    let parts = QubitSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 45);
    assert_eq!(parts.sequence(), 9);
    assert_eq!(parts.host(), 7);
}

#[test]
fn test_generate_at_rejects_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        7,
        epoch,
        epoch + Duration::from_millis(123),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    let time = epoch - Duration::from_nanos(1);

    assert!(matches!(
        generator.generate_at(time, 0),
        Err(IdError::TimeBeforeEpoch {
            time: actual_time,
            epoch: actual_epoch,
        }) if actual_time == time && actual_epoch == epoch
    ));
}

#[test]
fn test_qubit_snowflake_generator_accessors_return_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = QubitSnowflakeGenerator::builder(17)
        .mode(IdMode::Spread)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .max_clock_skew(Duration::from_millis(37))
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let expected_layout = QubitSnowflakeLayout::new(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        17,
    )
    .expect("layout should be valid");

    assert_eq!(generator.layout(), &expected_layout);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.max_clock_skew(), Duration::from_millis(37));
    assert_eq!(
        generator.expires_at(),
        expected_layout
            .expires_at(epoch)
            .expect("expiration should be representable")
    );
}

#[test]
fn test_qubit_snowflake_generator_increments_sequence_in_same_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        3,
        epoch,
        epoch + Duration::from_millis(10),
        DEFAULT_MAX_CLOCK_SKEW,
    );

    let first = generator.generate().expect("first ID should generate");
    let second = generator.generate().expect("second ID should generate");
    let first_parts = QubitSnowflakeLayout::decode(first);
    let second_parts = QubitSnowflakeLayout::decode(second);

    assert_eq!(first_parts.timestamp(), 10);
    assert_eq!(second_parts.timestamp(), 10);
    assert_eq!(first_parts.sequence(), 0);
    assert_eq!(second_parts.sequence(), 1);
}

#[test]
fn test_qubit_snowflake_generator_supports_concurrent_shared_access() {
    const WORKERS: usize = 128;

    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        3,
        epoch,
        epoch + Duration::from_millis(10),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    let generator = Arc::new(generator);
    let workers = (0..WORKERS)
        .map(|_| {
            let generator = Arc::clone(&generator);
            thread::spawn(move || generator.generate())
        })
        .collect::<Vec<_>>();
    let generated = workers
        .into_iter()
        .map(|worker| {
            worker
                .join()
                .expect("worker should finish")
                .expect("ID should generate")
        })
        .collect::<HashSet<_>>();

    assert_eq!(generated.len(), WORKERS);
}

#[test]
fn test_qubit_snowflake_generator_reports_large_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Millisecond,
        3,
        epoch,
        epoch + Duration::from_millis(10),
        Duration::ZERO,
    );
    generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));

    assert!(matches!(
        generator.generate(),
        Err(IdError::ClockMovedBackwards {
            last_elapsed,
            current_elapsed,
            skew,
            max_skew,
        }) if last_elapsed == Duration::from_millis(10)
            && current_elapsed == Duration::from_millis(9)
            && skew == Duration::from_millis(1)
            && max_skew == Duration::ZERO
    ));
}

#[test]
fn test_qubit_snowflake_generator_reports_runtime_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = QubitSnowflakeLayout::new(
        IdMode::Sequential,
        TimestampPrecision::Second,
        7,
    )
    .expect("layout should be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("expiration should be representable");
    let (generator, time) = build_generator(
        TimestampPrecision::Second,
        7,
        epoch,
        expires_at - Duration::from_nanos(1),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    assert_eq!(generator.expires_at(), expires_at);
    time.reanchor(expires_at);

    assert!(matches!(
        generator.generate(),
        Err(IdError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}

#[test]
fn test_qubit_snowflake_generator_rejects_expired_explicit_time() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = QubitSnowflakeLayout::new(
        IdMode::Sequential,
        TimestampPrecision::Second,
        7,
    )
    .expect("layout should be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("expiration should be representable");
    let (generator, _time) = build_generator(
        TimestampPrecision::Second,
        7,
        epoch,
        expires_at - Duration::from_nanos(1),
        DEFAULT_MAX_CLOCK_SKEW,
    );

    assert!(matches!(
        generator.generate_at(expires_at, 0),
        Err(IdError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}

#[test]
fn test_qubit_snowflake_generator_waits_with_injected_timer() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Second)
            .epoch(epoch)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid"),
    );
    let worker_generator = Arc::clone(&generator);
    let worker = thread::spawn(move || worker_generator.generate());

    time.advance_to_next_deadline();
    let id = worker
        .join()
        .expect("worker should finish")
        .expect("next slice should allocate");
    let parts = QubitSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_preserves_wait_failure_source() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .timer(Arc::new(FailingTimer::new()))
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.generate(),
        Err(IdError::WaitFailed {
            source: TimeError::InstantOverflow,
        })
    ));
}
