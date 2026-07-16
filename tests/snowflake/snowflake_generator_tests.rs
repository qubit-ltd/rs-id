// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the classic Snowflake generator.

use std::collections::HashSet;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_clock::FixedWallClock;
use qubit_id::{
    GenerationOutcome,
    IdError,
    IdGenerator,
    RestartPolicy,
    SnowflakeGenerator,
    SnowflakeLayout,
};

use crate::support::{
    ManualTime,
    PanickingWallClock,
};

#[test]
fn test_snowflake_generator_exposes_layout_and_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::builder(513)
        .epoch(epoch)
        .build()
        .expect("node id should be valid");

    assert_eq!(generator.layout().node_id(), 513);
    assert_eq!(generator.epoch(), epoch);
}

#[test]
fn test_snowflake_generator_rejects_invalid_node() {
    assert!(matches!(
        SnowflakeGenerator::new(1_024),
        Err(IdError::NodeOutOfRange {
            node_id: 1_024,
            max: 1_023,
        })
    ));
}

#[test]
fn test_snowflake_generator_next_string_uses_numeric_string() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(Arc::new(FixedWallClock::new(
            epoch + Duration::from_millis(77),
        )))
        .build()
        .expect("configuration should be valid");

    let id = generator.next_id().expect("id should generate");
    let next_string = generator
        .next_string()
        .expect("string id should generate after numeric id");

    assert_eq!(SnowflakeLayout::decode(id).timestamp(), 77);
    assert_eq!(next_string, (id + 1).to_string());
}

#[test]
fn test_snowflake_generator_reports_clock_backwards() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    generator.next_id().expect("first id should generate");
    time.reanchor(epoch + Duration::from_millis(9));

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
fn test_snowflake_generator_detects_raw_rollback_inside_millisecond() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_500));
    let generator = SnowflakeGenerator::builder(11)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator
            .try_next_id()
            .expect("first allocation should succeed"),
        GenerationOutcome::Generated(_)
    ));
    time.reanchor(epoch + Duration::from_micros(10_400));

    assert!(matches!(
        generator.try_next_id(),
        Err(IdError::ClockMovedBackwards {
            last_elapsed,
            current_elapsed,
            skew,
            max_skew,
        }) if last_elapsed == Duration::from_micros(10_500)
            && current_elapsed == Duration::from_micros(10_400)
            && skew == Duration::from_micros(100)
            && max_skew == Duration::ZERO
    ));
}

#[test]
fn test_snowflake_generator_wait_next_slice_delays_first_allocation() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = SnowflakeGenerator::builder(11)
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
        GenerationOutcome::RetryAfter(Duration::from_micros(750))
    );
    time.advance(Duration::from_micros(750));
    let id = match generator.try_next_id().expect("next slice should allocate")
    {
        GenerationOutcome::Generated(id) => id,
        GenerationOutcome::RetryAfter(duration) => {
            panic!("unexpected retry after {duration:?}")
        }
    };
    let parts = SnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_snowflake_generator_next_id_uses_injected_blocking_sleeper() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = Arc::new(
        SnowflakeGenerator::builder(11)
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
    let parts = SnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_snowflake_generator_reports_rollback_while_waiting() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
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
fn test_snowflake_generator_waits_when_sequence_overflows() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = Arc::new(
        SnowflakeGenerator::builder(9)
            .epoch(epoch)
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );

    for expected_sequence in 0..=4_095 {
        let id = generator.next_id().expect("id should generate");
        assert_eq!(SnowflakeLayout::decode(id).sequence(), expected_sequence);
    }
    assert_eq!(
        generator
            .try_next_id()
            .expect("sequence exhaustion should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_micros(750))
    );
    let worker_generator = Arc::clone(&generator);
    let worker = thread::spawn(move || worker_generator.next_id());
    time.advance_to_next_deadline();
    let wrapped = worker
        .join()
        .expect("generator worker should finish")
        .expect("generator should wait for the next millisecond");

    let parts = SnowflakeLayout::decode(wrapped);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_snowflake_generator_concurrent_overflow_is_unique() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = Arc::new(
        SnowflakeGenerator::builder(9)
            .epoch(epoch)
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );

    for _ in 0..=generator.layout().max_sequence() {
        generator.next_id().expect("id should generate");
    }

    let workers = (0..2)
        .map(|_| {
            let generator = Arc::clone(&generator);
            thread::spawn(move || generator.next_id())
        })
        .collect::<Vec<_>>();
    time.advance_to_next_deadline_after_waiters(2);

    let ids = workers
        .into_iter()
        .map(|worker| {
            worker
                .join()
                .expect("worker should finish")
                .expect("id should generate after the clock advances")
        })
        .collect::<Vec<_>>();
    let timestamps = ids
        .iter()
        .map(|id| SnowflakeLayout::decode(*id).timestamp())
        .collect::<HashSet<_>>();
    let sequences = ids
        .iter()
        .map(|id| SnowflakeLayout::decode(*id).sequence())
        .collect::<HashSet<_>>();

    assert_eq!(timestamps, HashSet::from([11]));
    assert_eq!(sequences, HashSet::from([0, 1]));
}

#[test]
fn test_snowflake_generator_same_node_restart_can_repeat_id() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let first_generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .build()
        .expect("configuration should be valid");
    let first = first_generator.next_id().expect("first id should generate");
    let second_generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .build()
        .expect("configuration should be valid");
    let second = second_generator
        .next_id()
        .expect("replacement generator should generate immediately");

    assert_eq!(first, second);
}

#[test]
fn test_snowflake_generator_first_id_uses_current_time_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(Arc::new(FixedWallClock::new(
            epoch + Duration::from_millis(10),
        )))
        .build()
        .expect("configuration should be valid");

    let id = generator
        .next_id()
        .expect("first id should generate immediately");

    let parts = SnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 10);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_snowflake_generator_recovers_after_clock_panics() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(Arc::new(PanickingWallClock::new(
            1,
            epoch + Duration::from_millis(10),
        )))
        .build()
        .expect("configuration should be valid");

    let panic = catch_unwind(AssertUnwindSafe(|| generator.next_id()));
    assert!(panic.is_err());

    let id = generator
        .next_id()
        .expect("generator should recover after the clock panic");
    let parts = SnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 10);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_snowflake_generator_reports_timestamp_overflow_from_clock() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let expires_at = epoch + Duration::from_millis(1_u64 << 41);
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .build()
        .expect("configuration should be valid");
    time.reanchor(expires_at + Duration::from_millis(1));

    assert!(matches!(
        generator.next_id(),
        Err(IdError::TimestampOverflow {
            timestamp,
            max,
        }) if timestamp == generator.layout().max_timestamp() + 2
            && max == generator.layout().max_timestamp()
    ));
}

#[test]
fn test_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::builder(9)
        .epoch(epoch)
        .wall_clock(Arc::new(FixedWallClock::new(
            epoch - Duration::from_millis(1),
        )))
        .build()
        .expect("configuration should be valid");

    let time = epoch - Duration::from_millis(1);
    assert!(matches!(
        generator.next_id(),
        Err(IdError::TimeBeforeEpoch {
            time: actual_time,
            epoch: actual_epoch,
        }) if actual_time == time && actual_epoch == epoch
    ));
}
