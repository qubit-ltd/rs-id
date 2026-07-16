// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the Qubit snowflake generator.

use std::collections::HashSet;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::thread;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_clock::TimeError;
use qubit_id::{
    DEFAULT_MAX_CLOCK_SKEW,
    GenerationOutcome,
    IdError,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    RestartPolicy,
    TimestampPrecision,
};

use crate::support::{
    ClosureWallClock,
    FailingBlockingSleeper,
    ManualTime,
};

#[test]
fn test_qubit_snowflake_generator_try_next_id_returns_retry_after_on_overflow()
{
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        7,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");

    for expected_sequence in 0..=generator.layout().max_sequence() {
        let id = match generator
            .try_next_id()
            .expect("allocation should succeed before overflow")
        {
            GenerationOutcome::Generated(id) => id,
            GenerationOutcome::RetryAfter(duration) => {
                panic!("unexpected retry after {duration:?}")
            }
        };
        assert_eq!(
            QubitSnowflakeLayout::decode(id).sequence(),
            expected_sequence
        );
    }

    assert_eq!(
        generator
            .try_next_id()
            .expect("sequence overflow should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(1))
    );
}

/// Builds a Qubit generator with an injected clock for deterministic tests.
///
/// `F` is the thread-safe wall-clock closure type owned by the fixture.
///
/// # Arguments
///
/// * `mode` - Qubit ordering mode to configure.
/// * `precision` - Timestamp precision to configure.
/// * `host` - Host identifier to encode.
/// * `epoch` - Timestamp origin to configure.
/// * `max_clock_skew` - Largest raw rollback that may be retried.
/// * `clock` - Deterministic wall-clock closure.
///
/// # Returns
///
/// A configured Qubit generator.
///
/// # Errors
///
/// Returns [`IdError::HostOutOfRange`] when `host` is invalid.
fn build_generator<F>(
    mode: IdMode,
    precision: TimestampPrecision,
    host: u64,
    epoch: SystemTime,
    max_clock_skew: Duration,
    clock: F,
) -> Result<QubitSnowflakeGenerator, IdError>
where
    F: Fn() -> SystemTime + Send + Sync + 'static,
{
    QubitSnowflakeGenerator::builder(host)
        .mode(mode)
        .precision(precision)
        .epoch(epoch)
        .max_clock_skew(max_clock_skew)
        .wall_clock(Arc::new(ClosureWallClock::new(clock)))
        .build()
}

#[test]
fn test_generate_at_matches_layout_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        7,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch + Duration::from_millis(123),
    )
    .expect("configuration should be valid");

    let id = generator
        .generate_at(epoch + Duration::from_millis(45), 9)
        .expect("timestamp and sequence should be valid");

    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 45);
    assert_eq!(parts.sequence(), 9);
    assert_eq!(parts.host(), 7);
    assert_eq!(generator.epoch(), epoch);
}

#[test]
fn test_qubit_snowflake_generator_accessors_return_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        17,
        epoch,
        Duration::from_millis(37),
        move || epoch + Duration::from_millis(100),
    )
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
}

#[test]
fn test_qubit_snowflake_generator_next_id_increments_sequence_in_same_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");

    assert_eq!(QubitSnowflakeLayout::decode(first).timestamp(), 10);
    assert_eq!(QubitSnowflakeLayout::decode(second).timestamp(), 10);
    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
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
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        Duration::ZERO,
        move || {
            epoch + Duration::from_millis(clock_millis.load(Ordering::SeqCst))
        },
    )
    .expect("configuration should be valid");

    generator.next_id().expect("first id should generate");
    current_millis.store(9, Ordering::SeqCst);

    assert!(matches!(
        generator.next_id(),
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
fn test_qubit_snowflake_generator_detects_raw_rollback_inside_time_slice() {
    let current_millis = Arc::new(AtomicU64::new(10_500));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Second,
        3,
        epoch,
        Duration::from_millis(100),
        move || {
            epoch + Duration::from_millis(clock_millis.load(Ordering::SeqCst))
        },
    )
    .expect("configuration should be valid");

    assert!(matches!(
        generator
            .try_next_id()
            .expect("first allocation should succeed"),
        GenerationOutcome::Generated(_)
    ));
    current_millis.store(10_400, Ordering::SeqCst);

    assert_eq!(
        generator
            .try_next_id()
            .expect("small raw rollback should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(100))
    );
}

#[test]
fn test_qubit_snowflake_generator_wait_next_slice_delays_first_allocation() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(
        generator
            .try_next_id()
            .expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(750))
    );
    time.advance(Duration::from_millis(749));
    assert_eq!(
        generator
            .try_next_id()
            .expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(1))
    );
    time.advance(Duration::from_millis(1));
    let id = match generator.try_next_id().expect("next slice should allocate")
    {
        GenerationOutcome::Generated(id) => id,
        GenerationOutcome::RetryAfter(duration) => {
            panic!("unexpected retry after {duration:?}")
        }
    };
    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_wait_next_slice_can_repeat_after_cross_restart_clock_rollback()
 {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let predecessor_time = ManualTime::new(epoch + Duration::from_secs(10));
    let predecessor = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .wall_clock(predecessor_time.wall_clock())
        .build()
        .expect("predecessor configuration should be valid");
    let predecessor_id = predecessor
        .next_id()
        .expect("predecessor should allocate in slice ten");
    drop(predecessor);

    let replacement_time = ManualTime::new(epoch + Duration::from_secs(9));
    let replacement = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(replacement_time.wall_clock())
        .build()
        .expect("replacement configuration should be valid");
    assert_eq!(
        replacement
            .try_next_id()
            .expect("first replacement attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_secs(1))
    );

    replacement_time.advance(Duration::from_secs(1));
    let replacement_id = replacement
        .next_id()
        .expect("replacement should allocate after its startup fence");

    assert_eq!(replacement_id, predecessor_id);
}

#[test]
fn test_qubit_snowflake_generator_next_id_uses_injected_blocking_sleeper() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Second)
            .epoch(epoch)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );
    let worker_generator = Arc::clone(&generator);
    let worker = thread::spawn(move || worker_generator.next_id());

    time.advance_to_next_deadline();
    let id = worker
        .join()
        .expect("generator worker should finish")
        .expect("next slice should allocate");
    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_next_id_preserves_sleep_failure_source() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(Arc::new(FailingBlockingSleeper::new()))
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.next_id(),
        Err(IdError::SleepFailed {
            source: TimeError::InstantOverflow,
        })
    ));
}

#[test]
fn test_qubit_snowflake_generator_reports_rollback_while_waiting() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = QubitSnowflakeGenerator::builder(3)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .max_clock_skew(Duration::ZERO)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    for _ in 0..=generator.layout().max_sequence() {
        generator.next_id().expect("sequence should be available");
    }
    assert_eq!(
        generator
            .try_next_id()
            .expect("sequence exhaustion should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(1))
    );
    time.reanchor(epoch);

    assert!(matches!(
        generator.try_next_id(),
        Err(IdError::ClockMovedBackwards {
            last_elapsed,
            current_elapsed,
            skew,
            max_skew,
        }) if last_elapsed == Duration::from_millis(10)
            && current_elapsed == Duration::ZERO
            && skew == Duration::from_millis(10)
            && max_skew == Duration::ZERO
    ));
}

#[test]
fn test_qubit_snowflake_generator_waits_for_small_clock_backwards() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(3)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .max_clock_skew(Duration::from_millis(2))
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );

    let first = generator.next_id().expect("first id should generate");
    time.reanchor(epoch + Duration::from_millis(9));
    let worker_generator = Arc::clone(&generator);
    let worker = thread::spawn(move || worker_generator.next_id());
    time.advance_to_next_deadline();
    let second = worker
        .join()
        .expect("generator worker should finish")
        .expect("small clock skew should wait and retry");

    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
}

#[test]
fn test_qubit_snowflake_generator_waits_when_sequence_overflows() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(3)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );

    for expected_sequence in 0..=4_095 {
        let id = generator.next_id().expect("id should generate");
        assert_eq!(
            QubitSnowflakeLayout::decode(id).sequence(),
            expected_sequence
        );
    }
    let worker_generator = Arc::clone(&generator);
    let worker = thread::spawn(move || worker_generator.next_id());
    time.advance_to_next_deadline();
    let wrapped = worker
        .join()
        .expect("generator worker should finish")
        .expect("generator should wait for the next timestamp");

    assert_eq!(QubitSnowflakeLayout::decode(wrapped).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(wrapped).sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_first_id_uses_current_time_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");

    let id = generator
        .next_id()
        .expect("first id should generate immediately");

    assert_eq!(QubitSnowflakeLayout::decode(id).timestamp(), 10);
    assert_eq!(QubitSnowflakeLayout::decode(id).sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_concurrent_overflow_is_unique() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(3)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );

    for _ in 0..=generator.layout().max_sequence() {
        generator.next_id().expect("id should generate");
    }

    let mut workers = Vec::new();
    for _ in 0..2 {
        let generator = Arc::clone(&generator);
        workers.push(thread::spawn(move || generator.next_id()));
    }
    time.advance_to_next_deadline_after_waiters(2);

    let mut ids = Vec::new();
    for worker in workers {
        ids.push(
            worker
                .join()
                .expect("worker should finish")
                .expect("id should generate after the clock advances"),
        );
    }
    let timestamps = ids
        .iter()
        .map(|id| QubitSnowflakeLayout::decode(*id).timestamp())
        .collect::<HashSet<_>>();
    let sequences = ids
        .iter()
        .map(|id| QubitSnowflakeLayout::decode(*id).sequence())
        .collect::<HashSet<_>>();

    assert_eq!(timestamps, HashSet::from([11]));
    assert_eq!(sequences, HashSet::from([0, 1]));
}

#[test]
fn test_qubit_snowflake_generator_same_host_restart_can_repeat_id() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let first_generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");
    let first = first_generator.next_id().expect("first id should generate");
    let second_generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");
    let second = second_generator
        .next_id()
        .expect("replacement generator should generate immediately");

    assert_eq!(first, second);
}

#[test]
fn test_qubit_snowflake_generator_recovers_after_clock_panics() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || match clock_calls.fetch_add(1, Ordering::SeqCst) {
            0 => epoch + Duration::from_millis(10),
            1 => panic!("test clock panic"),
            _ => epoch + Duration::from_millis(10),
        },
    )
    .expect("configuration should be valid");

    let panic = catch_unwind(AssertUnwindSafe(|| generator.next_id()));
    assert!(
        panic.is_err(),
        "the first allocation clock call should panic"
    );

    let id = generator
        .next_id()
        .expect("generator should recover after the clock panic");
    assert_eq!(QubitSnowflakeLayout::decode(id).timestamp(), 10);
    assert_eq!(QubitSnowflakeLayout::decode(id).sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_reports_timestamp_overflow_from_time() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch,
    )
    .expect("configuration should be valid");
    let timestamp = generator.layout().max_timestamp() + 1;

    assert!(matches!(
        generator.generate_at(epoch + Duration::from_millis(timestamp), 0),
        Err(IdError::TimestampOverflow {
            timestamp: actual_timestamp,
            max,
        }) if actual_timestamp == timestamp
            && max == generator.layout().max_timestamp()
    ));
}

#[test]
fn test_qubit_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = build_generator(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_CLOCK_SKEW,
        move || epoch,
    )
    .expect("configuration should be valid");

    let time = epoch - Duration::from_millis(1);
    assert!(matches!(
        generator.generate_at(time, 0),
        Err(IdError::TimeBeforeEpoch {
            time: actual_time,
            epoch: actual_epoch,
        }) if actual_time == time && actual_epoch == epoch
    ));
}

#[test]
fn test_qubit_snowflake_generator_rejects_invalid_host_from_builder() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);

    assert!(matches!(
        build_generator(
            IdMode::Sequential,
            TimestampPrecision::Millisecond,
            512,
            epoch,
            DEFAULT_MAX_CLOCK_SKEW,
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
    let generator = Arc::new(
        QubitSnowflakeGenerator::new(11).expect("host should be valid"),
    );
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
