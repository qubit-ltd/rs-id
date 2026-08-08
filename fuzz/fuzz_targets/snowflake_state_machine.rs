// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzzes Qubit Snowflake allocation state across clock changes.

#![no_main]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use libfuzzer_sys::fuzz_target;
use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_clock::Timer;
use qubit_clock::WallClock;
use qubit_id::GenerationAttempt;
use qubit_id::Id;
use qubit_id::IdGenerationError;
use qubit_id::SnowflakeGenerator;
use qubit_id::TimestampPrecision;

/// Maximum number of state-machine operations interpreted from one input.
const MAX_OPERATIONS: usize = 256;
/// Raw rollback tolerated by the configured generator.
const MAX_CLOCK_SKEW: Duration = Duration::from_millis(5);
/// Initial offset after the epoch, leaving room for bounded rollbacks.
const INITIAL_ELAPSED: Duration = Duration::from_millis(100);

/// Builds a deterministic Qubit generator sharing the supplied manual clock.
///
/// # Parameters
///
/// * `clock` - Shared manual monotonic timeline for wall-time and timer use.
/// * `wall_clock` - Mutable wall-time projection sampled by the generator.
///
/// # Returns
///
/// A millisecond-precision generator with a five-millisecond rollback limit.
///
/// # Panics
///
/// Panics if the fixed fuzz configuration becomes invalid.
fn build_generator(
    clock: &Arc<ManualMonotonicClock>,
    wall_clock: Arc<dyn WallClock>,
) -> SnowflakeGenerator {
    let timer: Arc<dyn Timer> = clock.new_timer();
    let epoch = UNIX_EPOCH;
    SnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .max_clock_skew(MAX_CLOCK_SKEW)
        .wall_clock(wall_clock)
        .timer(timer)
        .build()
        .expect("fixed fuzz generator configuration must be valid")
}

/// Records a generated identifier and rejects a duplicate allocation.
///
/// # Parameters
///
/// * `id` - Identifier returned by the generator.
/// * `generated_ids` - Successful allocations observed for this instance.
///
/// # Panics
///
/// Panics when one generator instance returns an identifier more than once.
fn record_generated(id: Id, generated_ids: &mut BTreeSet<Id>) {
    assert!(
        generated_ids.insert(id),
        "one generator instance must not generate duplicate IDs",
    );
}

/// Requires a non-blocking allocation attempt to defer by a positive delay.
///
/// # Parameters
///
/// * `attempt` - Result of one non-blocking generation attempt.
///
/// # Panics
///
/// Panics when the attempt generates an ID or reports a zero retry delay.
fn require_positive_retry(
    attempt: Result<GenerationAttempt<Id>, IdGenerationError>,
) {
    match attempt {
        Ok(GenerationAttempt::RetryAfter { delay }) => {
            assert!(
                !delay.is_zero(),
                "a deferred generation attempt must report a positive delay",
            );
        }
        Ok(GenerationAttempt::Generated(id)) => {
            panic!(
                "expected deferred generation after rollback, generated {id}"
            );
        }
        Err(error) => {
            panic!("expected deferred generation after rollback: {error}")
        }
    }
}

/// Requires a non-blocking allocation attempt to reject an excessive rollback.
///
/// # Parameters
///
/// * `attempt` - Result of one non-blocking generation attempt.
///
/// # Panics
///
/// Panics when the excessive rollback does not return `ClockMovedBackwards`.
fn require_large_rollback_error(
    attempt: Result<GenerationAttempt<Id>, IdGenerationError>,
) {
    match attempt {
        Err(IdGenerationError::ClockMovedBackwards {
            skew, max_skew, ..
        }) => {
            assert!(
                skew > max_skew,
                "clock rollback error must exceed its configured tolerance",
            );
        }
        Ok(GenerationAttempt::Generated(id)) => {
            panic!("excessive rollback generated {id} instead of failing");
        }
        Ok(GenerationAttempt::RetryAfter { delay }) => {
            panic!(
                "excessive rollback retried after {delay:?} instead of failing"
            );
        }
        Err(error) => panic!("unexpected excessive rollback error: {error}"),
    }
}

fuzz_target!(|input: &[u8]| {
    let operations = &input[..input.len().min(MAX_OPERATIONS)];
    let epoch = UNIX_EPOCH;
    let mut current_time = epoch
        .checked_add(INITIAL_ELAPSED)
        .expect("fixed initial fuzz time must be representable");
    let clock = ManualMonotonicClock::new_shared();
    let wall_clock = clock.new_wall_clock(current_time);
    let generator = build_generator(&clock, wall_clock.clone());
    let mut generated_ids = BTreeSet::new();

    let first_id = match generator.try_generate() {
        Ok(GenerationAttempt::Generated(id)) => id,
        Ok(GenerationAttempt::RetryAfter { delay }) => {
            panic!(
                "the default Immediate policy must not defer first generation: {delay:?}"
            );
        }
        Err(error) => panic!("initial generation must succeed: {error}"),
    };
    record_generated(first_id, &mut generated_ids);
    let mut last_observed_time = current_time;

    for operation in operations {
        match operation >> 6 {
            0 => match generator.try_generate() {
                Ok(GenerationAttempt::Generated(id)) => {
                    record_generated(id, &mut generated_ids)
                }
                Ok(GenerationAttempt::RetryAfter { delay }) => {
                    assert!(
                        !delay.is_zero(),
                        "a deferred generation attempt must report a positive delay",
                    );
                }
                Err(error) => {
                    panic!("normal generation must not fail: {error}")
                }
            },
            1 => {
                let advance =
                    Duration::from_millis(u64::from(operation & 0x07) + 1);
                clock
                    .advance(advance)
                    .expect("bounded manual clock advance must succeed");
                current_time = current_time
                    .checked_add(advance)
                    .expect("bounded fuzz time advance must be representable");
            }
            2 => {
                let rollback =
                    Duration::from_millis(u64::from(operation & 0x3f) % 5 + 1);
                let rollback_time =
                    last_observed_time.checked_sub(rollback).expect(
                        "initial fuzz offset must contain bounded rollback",
                    );
                wall_clock.reanchor(rollback_time);
                require_positive_retry(generator.try_generate());
                wall_clock.reanchor(current_time);
            }
            _ => {
                let rollback =
                    Duration::from_millis(u64::from(operation & 0x3f) % 8 + 6);
                let rollback_time =
                    last_observed_time.checked_sub(rollback).expect(
                        "initial fuzz offset must contain bounded rollback",
                    );
                wall_clock.reanchor(rollback_time);
                require_large_rollback_error(generator.try_generate());
                wall_clock.reanchor(current_time);
            }
        }

        if operation >> 6 == 0 {
            last_observed_time = current_time;
        }
    }
});
