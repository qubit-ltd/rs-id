// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the Sonyflake-style generator.

use std::collections::HashSet;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_clock::FixedWallClock;
use qubit_id::{
    GenerationOutcome,
    IdError,
    IdGenerator,
    RestartPolicy,
    SonyflakeGenerator,
};

use crate::support::{
    ManualTime,
    PanickingWallClock,
};

/// Builds a Sonyflake generator with an injected clock for deterministic tests.
///
/// # Arguments
///
/// * `machine_id` - Machine identifier to encode.
/// * `bits_sequence` - Sequence field width to configure.
/// * `bits_machine` - Machine field width to configure.
/// * `time_unit` - Duration represented by one elapsed-time unit.
/// * `start_time` - Elapsed-time origin to configure.
/// * `current_time` - Fixed wall time observed by the generator.
///
/// # Returns
///
/// A configured Sonyflake-style generator.
///
/// # Errors
///
/// Returns an [`IdError`] when any supplied configuration value is invalid.
fn build_generator(
    machine_id: u64,
    bits_sequence: u8,
    bits_machine: u8,
    time_unit: Duration,
    start_time: SystemTime,
    current_time: SystemTime,
) -> Result<SonyflakeGenerator, IdError> {
    SonyflakeGenerator::builder(machine_id)
        .bits_sequence(bits_sequence)
        .bits_machine(bits_machine)
        .time_unit(time_unit)
        .start_time(start_time)
        .wall_clock(Arc::new(FixedWallClock::new(current_time)))
        .build()
}

#[test]
fn test_sonyflake_generator_default_layout_matches_sonyflake() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = build_generator(
        0x1234,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        epoch + Duration::from_millis(120),
    )
    .expect("configuration should be valid");

    let id = generator
        .compose(12, 7, 0x1234)
        .expect("parts should be valid");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
    assert_eq!(id, (12_u64 << 24) | (7_u64 << 16) | 0x1234);
    assert_eq!(generator.extract_elapsed_time(id), 12);
    assert_eq!(generator.extract_sequence(id), 7);
    assert_eq!(generator.extract_machine_id(id), 0x1234);
}

#[test]
fn test_sonyflake_generator_accessors_return_configuration() {
    let start_time = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time_unit = Duration::from_millis(5);
    let generator = build_generator(
        17,
        7,
        5,
        time_unit,
        start_time,
        start_time + Duration::from_millis(100),
    )
    .expect("configuration should be valid");

    assert_eq!(generator.machine_id(), 17);
    assert_eq!(generator.start_time(), start_time);
    assert_eq!(generator.time_unit(), time_unit);
    assert_eq!(generator.bits_time(), 51);
    assert_eq!(generator.bits_sequence(), 7);
    assert_eq!(generator.bits_machine(), 5);
}

#[test]
fn test_sonyflake_generator_new_uses_default_layout() {
    let generator =
        SonyflakeGenerator::new(1).expect("default machine id should be valid");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
}

#[test]
fn test_sonyflake_generator_first_id_uses_current_time_unit() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = build_generator(
        1,
        8,
        16,
        Duration::from_millis(1),
        epoch,
        epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");

    let id = generator
        .next_id()
        .expect("first id should generate immediately");

    assert_eq!(generator.extract_elapsed_time(id), 10);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_sonyflake_generator_zero_bits_select_defaults() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator =
        build_generator(1, 0, 0, Duration::from_millis(10), epoch, epoch)
            .expect("zero bit lengths should select defaults");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
}

#[test]
fn test_sonyflake_generator_next_id_waits_for_physical_next_time_unit() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time = ManualTime::new(epoch + Duration::from_millis(5));
    let generator = SonyflakeGenerator::builder(1)
        .bits_sequence(1)
        .bits_machine(1)
        .time_unit(Duration::from_millis(1))
        .start_time(epoch)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");
    let generator = Arc::new(generator);
    let third_generator = Arc::clone(&generator);
    let third = thread::spawn(move || third_generator.next_id());

    time.advance_to_next_deadline();
    let third = third
        .join()
        .expect("worker should finish")
        .expect("third id should generate");

    assert_eq!(generator.extract_elapsed_time(first), 5);
    assert_eq!(generator.extract_sequence(first), 0);
    assert_eq!(generator.extract_elapsed_time(second), 5);
    assert_eq!(generator.extract_sequence(second), 1);
    assert_eq!(generator.extract_elapsed_time(third), 6);
    assert_eq!(generator.extract_sequence(third), 0);
}

#[test]
fn test_sonyflake_generator_concurrent_overflow_is_unique() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = Arc::new(
        SonyflakeGenerator::builder(1)
            .bits_sequence(1)
            .bits_machine(1)
            .time_unit(Duration::from_millis(1))
            .start_time(epoch)
            .wall_clock(time.wall_clock())
            .blocking_sleeper(time.blocking_sleeper())
            .build()
            .expect("configuration should be valid"),
    );

    for _ in 0..=generator.max_sequence() {
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
        .map(|id| generator.extract_elapsed_time(*id))
        .collect::<HashSet<_>>();
    let sequences = ids
        .iter()
        .map(|id| generator.extract_sequence(*id))
        .collect::<HashSet<_>>();

    assert_eq!(timestamps, HashSet::from([11]));
    assert_eq!(sequences, HashSet::from([0, 1]));
}

#[test]
fn test_sonyflake_generator_same_machine_restart_can_repeat_id() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let first_generator = build_generator(
        1,
        1,
        1,
        Duration::from_millis(1),
        epoch,
        epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");
    let first = first_generator.next_id().expect("first id should generate");
    let second_generator = build_generator(
        1,
        1,
        1,
        Duration::from_millis(1),
        epoch,
        epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");
    let second = second_generator
        .next_id()
        .expect("replacement generator should generate immediately");

    assert_eq!(first, second);
}

#[test]
fn test_sonyflake_generator_rejects_invalid_settings_and_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);

    assert!(matches!(
        SonyflakeGenerator::builder(65_536)
            .start_time(epoch)
            .build(),
        Err(IdError::MachineIdOutOfRange {
            machine_id: 65_536,
            max: 65_535,
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::builder(1)
            .bits_sequence(31)
            .bits_machine(1)
            .time_unit(Duration::from_millis(10))
            .start_time(epoch)
            .build(),
        Err(IdError::InvalidBitLength {
            name: "sequence",
            ..
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::builder(1)
            .bits_sequence(30)
            .bits_machine(2)
            .time_unit(Duration::from_millis(10))
            .start_time(epoch)
            .build(),
        Err(IdError::InvalidBitLength {
            name: "time",
            bits: 31,
            ..
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::builder(1)
            .bits_sequence(8)
            .bits_machine(16)
            .time_unit(Duration::from_millis(10))
            .start_time(epoch + Duration::from_millis(1))
            .wall_clock(Arc::new(FixedWallClock::new(epoch)))
            .build(),
        Err(IdError::StartTimeAhead {
            start_time,
            current_time,
        }) if start_time == epoch + Duration::from_millis(1)
            && current_time == epoch
    ));
    assert!(matches!(
        SonyflakeGenerator::builder(1)
            .bits_sequence(8)
            .bits_machine(16)
            .time_unit(Duration::from_nanos(1))
            .start_time(epoch)
            .build(),
        Err(IdError::InvalidTimeUnit {
            nanos: 1,
            min_nanos: 1_000_000,
        })
    ));

    let generator = SonyflakeGenerator::builder(1)
        .start_time(epoch)
        .build()
        .expect("machine id should be valid");
    assert!(matches!(
        generator.compose(generator.max_elapsed_time() + 1, 0, 1),
        Err(IdError::TimestampOverflow {
            timestamp,
            max,
        }) if timestamp == generator.max_elapsed_time() + 1
            && max == generator.max_elapsed_time()
    ));
    assert!(matches!(
        generator.compose(0, generator.max_sequence() + 1, 1),
        Err(IdError::SequenceOverflow {
            sequence,
            max,
        }) if sequence == generator.max_sequence() + 1
            && max == generator.max_sequence()
    ));
    assert!(matches!(
        generator.compose(0, 0, generator.max_machine_id() + 1),
        Err(IdError::MachineIdOutOfRange {
            machine_id,
            max,
        }) if machine_id == generator.max_machine_id() + 1
            && max == generator.max_machine_id()
    ));
}

#[test]
fn test_sonyflake_generator_string_output_is_numeric() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = build_generator(
        7,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        epoch + Duration::from_millis(10),
    )
    .expect("configuration should be valid");

    let id = generator.next_id().expect("id should generate");

    assert_eq!(
        generator
            .next_string()
            .expect("string id should generate after numeric id"),
        (id + (1_u64 << 16)).to_string()
    );
}

#[test]
fn test_sonyflake_generator_reports_clock_backwards() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SonyflakeGenerator::builder(7)
        .bits_sequence(8)
        .bits_machine(16)
        .time_unit(Duration::from_millis(1))
        .start_time(epoch)
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
fn test_sonyflake_generator_detects_raw_rollback_inside_time_unit() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(25));
    let generator = SonyflakeGenerator::builder(13)
        .bits_sequence(2)
        .time_unit(Duration::from_millis(10))
        .start_time(start_time)
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
    time.reanchor(start_time + Duration::from_millis(24));

    assert!(matches!(
        generator.try_next_id(),
        Err(IdError::ClockMovedBackwards {
            last_elapsed,
            current_elapsed,
            skew,
            max_skew,
        }) if last_elapsed == Duration::from_millis(25)
            && current_elapsed == Duration::from_millis(24)
            && skew == Duration::from_millis(1)
            && max_skew == Duration::ZERO
    ));
}

#[test]
fn test_sonyflake_generator_wait_next_slice_delays_first_allocation() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(25));
    let generator = SonyflakeGenerator::builder(13)
        .bits_sequence(2)
        .time_unit(Duration::from_millis(10))
        .start_time(start_time)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(
        generator
            .try_next_id()
            .expect("attempt should be retryable"),
        GenerationOutcome::RetryAfter(Duration::from_millis(5))
    );
    time.advance(Duration::from_millis(5));
    let id = match generator.try_next_id().expect("next unit should allocate") {
        GenerationOutcome::Generated(id) => id,
        GenerationOutcome::RetryAfter(duration) => {
            panic!("unexpected retry after {duration:?}")
        }
    };
    assert_eq!(generator.extract_elapsed_time(id), 3);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_sonyflake_generator_next_id_uses_injected_blocking_sleeper() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(25));
    let generator = Arc::new(
        SonyflakeGenerator::builder(13)
            .bits_sequence(2)
            .time_unit(Duration::from_millis(10))
            .start_time(start_time)
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
        .expect("next unit should allocate");
    assert_eq!(generator.extract_elapsed_time(id), 3);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_sonyflake_generator_reports_rollback_while_waiting() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SonyflakeGenerator::builder(1)
        .bits_sequence(1)
        .bits_machine(1)
        .time_unit(Duration::from_millis(1))
        .start_time(epoch)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    generator
        .next_id()
        .expect("first sequence should be available");
    generator
        .next_id()
        .expect("second sequence should be available");
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
fn test_sonyflake_generator_recovers_after_clock_panics() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::builder(7)
        .bits_sequence(8)
        .bits_machine(16)
        .time_unit(Duration::from_millis(1))
        .start_time(epoch)
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
    assert_eq!(generator.extract_elapsed_time(id), 10);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_sonyflake_generator_reports_time_before_epoch_after_construction() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let manual_time = ManualTime::new(epoch);
    let generator = SonyflakeGenerator::builder(7)
        .bits_sequence(8)
        .bits_machine(16)
        .time_unit(Duration::from_millis(10))
        .start_time(epoch)
        .wall_clock(manual_time.wall_clock())
        .blocking_sleeper(manual_time.blocking_sleeper())
        .build()
        .expect("construction clock should be at epoch");

    manual_time.reanchor(epoch - Duration::from_millis(1));

    let time = epoch - Duration::from_millis(1);
    assert!(matches!(
        generator.next_id(),
        Err(IdError::TimeBeforeEpoch {
            time: actual_time,
            epoch: actual_epoch,
        }) if actual_time == time && actual_epoch == epoch
    ));
}

#[test]
fn test_sonyflake_generator_reports_timestamp_overflow_from_clock() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = build_generator(
        7,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        epoch + Duration::from_millis((1_u64 << 39) * 10),
    )
    .expect("configuration should be valid");

    assert!(matches!(
        generator.next_id(),
        Err(IdError::TimestampOverflow {
            timestamp,
            max,
        }) if timestamp == generator.max_elapsed_time() + 1
            && max == generator.max_elapsed_time()
    ));
}
