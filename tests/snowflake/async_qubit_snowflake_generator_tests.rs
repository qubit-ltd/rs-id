// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the asynchronous Qubit Snowflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_clock::{
    TimeError,
    TimerUnavailableError,
    test_util::{
        FaultInjectingTimer,
        TimerFailurePoint,
    },
};
use qubit_id::{
    DEFAULT_MAX_CLOCK_SKEW,
    GenerationAttempt,
    IdError,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    RestartPolicy,
    TimestampPrecision,
};

use crate::support::{
    CompletionFailingTimer,
    ManualTime,
};

/// Builds an asynchronous Qubit generator on one manual timeline.
///
/// # Parameters
///
/// * `precision` - Timestamp precision used by generated IDs.
/// * `epoch` - Timestamp origin.
/// * `now` - Initial wall time.
/// * `max_clock_skew` - Largest tolerated wall-clock rollback.
///
/// # Returns
///
/// The configured generator and time controller.
fn build_generator(
    precision: TimestampPrecision,
    epoch: SystemTime,
    now: SystemTime,
    max_clock_skew: Duration,
) -> (QubitSnowflakeGenerator, ManualTime) {
    let time = ManualTime::new(now);
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(precision)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .max_clock_skew(max_clock_skew)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    (generator, time)
}

#[test]
fn test_async_qubit_snowflake_generator_convenience_api() {
    let generator = QubitSnowflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().host(), 17);
    assert_eq!(generator.max_clock_skew(), DEFAULT_MAX_CLOCK_SKEW);
    assert_eq!(
        generator.expires_at(),
        generator
            .layout()
            .expires_at(generator.epoch())
            .expect("default expiration should be representable")
    );

    let id = generator
        .compose_at(generator.epoch(), 3)
        .expect("the first timestamp should be representable");
    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.host(), 17);
    assert_eq!(parts.timestamp(), 0);
    assert_eq!(parts.sequence(), 3);
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_increments_sequence() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        epoch,
        epoch + Duration::from_millis(10),
        DEFAULT_MAX_CLOCK_SKEW,
    );

    let first = generator
        .generate_async()
        .await
        .expect("first ID should generate");
    let second = generator
        .generate_async()
        .await
        .expect("second ID should generate");

    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
}

#[tokio::test]
async fn test_unified_qubit_generator_shares_state_across_call_paths() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let first = generator
        .try_generate()
        .expect("try allocation should succeed");
    let first = match first {
        GenerationAttempt::Generated(id) => id,
        GenerationAttempt::RetryAfter { .. } => {
            panic!("explicit immediate policy should allocate")
        }
    };
    let second = generator
        .generate_async()
        .await
        .expect("async allocation should succeed");

    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_waits_for_sequence_capacity() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Millisecond,
        epoch,
        epoch + Duration::from_millis(10),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    let generator = Arc::new(generator);
    for _ in 0..=generator.layout().max_sequence() {
        generator
            .generate_async()
            .await
            .expect("sequence should remain available");
    }
    let worker_generator = Arc::clone(&generator);
    let worker =
        tokio::spawn(async move { worker_generator.generate_async().await });
    let deadline = time.advance_to_next_deadline_async().await;

    assert_eq!(deadline.elapsed_since_origin(), Duration::from_millis(1));
    let id = worker
        .await
        .expect("worker should finish")
        .expect("next time slice should allocate");
    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_wait_is_cancellation_safe() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Millisecond,
        epoch,
        epoch + Duration::from_millis(10),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    let generator = Arc::new(generator);
    for _ in 0..=generator.layout().max_sequence() {
        generator
            .generate_async()
            .await
            .expect("sequence should remain available");
    }
    let waiter_observer = time.wait_for_waiters_async(1);
    let worker_generator = Arc::clone(&generator);
    let worker =
        tokio::spawn(async move { worker_generator.generate_async().await });
    waiter_observer.await;
    assert_eq!(time.pending_waiters(), 1);

    worker.abort();
    let join_error = worker
        .await
        .expect_err("aborted generation should be cancelled");
    assert!(join_error.is_cancelled());
    assert_eq!(time.pending_waiters(), 0);

    time.advance(Duration::from_millis(1));
    let id = generator
        .generate_async()
        .await
        .expect("generator should remain usable after cancellation");
    assert_eq!(QubitSnowflakeLayout::decode(id).timestamp(), 11);
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_waits_for_small_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Millisecond,
        epoch,
        epoch + Duration::from_millis(10),
        Duration::from_millis(1),
    );
    let generator = Arc::new(generator);
    generator
        .generate_async()
        .await
        .expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));
    let worker_generator = Arc::clone(&generator);
    let worker =
        tokio::spawn(async move { worker_generator.generate_async().await });

    assert_eq!(
        time.advance_to_next_deadline_async()
            .await
            .elapsed_since_origin(),
        Duration::from_millis(1)
    );
    let id = worker
        .await
        .expect("worker should finish")
        .expect("generation should resume after rollback catches up");
    assert_eq!(QubitSnowflakeLayout::decode(id).sequence(), 1);
}

#[tokio::test]
async fn test_async_qubit_restart_fence_retries_the_baseline_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid"),
    );
    let worker_generator = Arc::clone(&generator);
    let worker =
        tokio::spawn(async move { worker_generator.generate_async().await });

    assert_eq!(
        time.wait_for_next_deadline_async()
            .await
            .elapsed_since_origin(),
        Duration::from_micros(750)
    );
    time.reanchor(epoch + Duration::from_millis(9));
    time.advance_to_next_deadline();

    assert_eq!(
        time.advance_to_next_deadline_async()
            .await
            .elapsed_since_origin(),
        Duration::from_micros(1_250)
    );

    assert_eq!(
        time.advance_to_next_deadline_async()
            .await
            .elapsed_since_origin(),
        Duration::from_millis(2)
    );

    let id = worker
        .await
        .expect("worker should finish")
        .expect("generation should resume after the baseline slice");
    assert_eq!(QubitSnowflakeLayout::decode(id).timestamp(), 11);
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_reports_large_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Millisecond,
        epoch,
        epoch + Duration::from_millis(10),
        Duration::ZERO,
    );
    generator
        .generate_async()
        .await
        .expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));

    assert!(matches!(
        generator.generate_async().await,
        Err(IdError::ClockMovedBackwards { skew, .. })
            if skew == Duration::from_millis(1)
    ));
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_preserves_wait_failure_source() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .timer(Arc::new(FaultInjectingTimer::new(
            TimerFailurePoint::Registration,
            || TimeError::InstantOverflow,
        )))
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.generate_async().await,
        Err(IdError::WaitFailed {
            source: TimeError::InstantOverflow,
        })
    ));
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_preserves_completion_failure_source()
 {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .timer(Arc::new(CompletionFailingTimer::new()))
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.generate_async().await,
        Err(IdError::WaitFailed {
            source: TimeError::TimerUnavailable {
                source: TimerUnavailableError::SchedulerWorkerTerminated,
            },
        })
    ));
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_reports_runtime_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = QubitSnowflakeLayout::new(
        qubit_id::IdMode::Sequential,
        TimestampPrecision::Second,
        7,
    )
    .expect("layout should be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("expiration should be representable");
    let (generator, time) = build_generator(
        TimestampPrecision::Second,
        epoch,
        expires_at - Duration::from_nanos(1),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    time.reanchor(expires_at);

    assert!(matches!(
        generator.generate_async().await,
        Err(IdError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}

#[tokio::test]
async fn test_async_qubit_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Second,
        epoch,
        epoch + Duration::from_secs(1),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    time.reanchor(epoch - Duration::from_secs(1));

    assert!(matches!(
        generator.generate_async().await,
        Err(IdError::TimeBeforeEpoch { time: observed, epoch: actual })
            if observed == epoch - Duration::from_secs(1) && actual == epoch
    ));
}
