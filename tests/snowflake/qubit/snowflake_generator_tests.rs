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
    BlockingIdGenerator,
    DEFAULT_MAX_CLOCK_SKEW,
    GenerationAttempt,
    Id,
    IdGenerationError,
    IdMode,
    RestartPolicy,
    SnowflakeGenerator,
    SnowflakeLayout,
    TimestampPrecision,
    TryIdGenerator,
};

use qubit_clock::{
    TimeError,
    test_util::{
        FaultInjectingTimer,
        TimerFailurePoint,
    },
};

use crate::support::ManualTime;

/// Builds a deterministic Qubit generator and its shared manual timeline.
///
/// # Parameters
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
) -> (SnowflakeGenerator, ManualTime) {
    let time = ManualTime::new(now);
    let generator = SnowflakeGenerator::builder(host)
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
fn test_snowflake_generator_new_uses_defaults() {
    let generator = SnowflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().host(), 17);
    assert_eq!(generator.max_clock_skew(), DEFAULT_MAX_CLOCK_SKEW);
}

#[test]
fn test_snowflake_wait_next_slice_retries_without_blocking() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::RetryAfter { delay })
            if delay == Duration::from_millis(1)
    ));
    time.advance(Duration::from_millis(1));
    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(_))
    ));
}

#[test]
fn test_snowflake_supports_nonblocking_trait_object() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator: Arc<
        dyn TryIdGenerator<Output = Id, Error = IdGenerationError>,
    > = Arc::new(
        SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Millisecond)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid"),
    );

    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(_))
    ));
}

#[test]
fn test_snowflake_supports_sync_trait_object() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        7,
        epoch,
        epoch + Duration::from_millis(10),
        DEFAULT_MAX_CLOCK_SKEW,
    );
    let generator: Box<
        dyn BlockingIdGenerator<Output = Id, Error = IdGenerationError>,
    > = Box::new(generator);

    let _ = generator
        .generate()
        .expect("trait-object generation should succeed");
}

mod inherent_api_tests {
    use super::TimestampPrecision;
    use super::build_generator;
    use std::time::{
        Duration,
        UNIX_EPOCH,
    };

    #[test]
    fn test_snowflake_generator_supports_inherent_generate() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let (generator, _time) = build_generator(
            TimestampPrecision::Millisecond,
            7,
            epoch,
            epoch + Duration::from_millis(10),
            Duration::from_millis(5),
        );

        let _id = generator
            .generate()
            .expect("inherent generation should succeed");
    }
}

#[test]
fn test_compose_at_matches_layout_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, _time) = build_generator(
        TimestampPrecision::Millisecond,
        7,
        epoch,
        epoch + Duration::from_millis(123),
        DEFAULT_MAX_CLOCK_SKEW,
    );

    let id = generator
        .compose_at(epoch + Duration::from_millis(45), 9)
        .expect("timestamp and sequence should be valid");
    let parts = SnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 45);
    assert_eq!(parts.sequence(), 9);
    assert_eq!(parts.host(), 7);
}

#[test]
fn test_compose_at_rejects_time_before_epoch() {
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
        generator.compose_at(time, 0),
        Err(IdGenerationError::TimeBeforeEpoch {
            time: actual_time,
            epoch: actual_epoch,
        }) if actual_time == time && actual_epoch == epoch
    ));
}

#[test]
fn test_snowflake_generator_accessors_return_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SnowflakeGenerator::builder(17)
        .mode(IdMode::Spread)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .max_clock_skew(Duration::from_millis(37))
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let expected_layout = SnowflakeLayout::new(
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
fn test_snowflake_generator_increments_sequence_in_same_slice() {
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
    let first_parts = SnowflakeLayout::decode(first);
    let second_parts = SnowflakeLayout::decode(second);

    assert_eq!(first_parts.timestamp(), 10);
    assert_eq!(second_parts.timestamp(), 10);
    assert_eq!(first_parts.sequence(), 0);
    assert_eq!(second_parts.sequence(), 1);
}

#[test]
fn test_snowflake_generator_supports_concurrent_shared_access() {
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
fn test_snowflake_generator_reports_large_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let (generator, time) = build_generator(
        TimestampPrecision::Millisecond,
        3,
        epoch,
        epoch + Duration::from_millis(10),
        Duration::ZERO,
    );
    let _ = generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));

    assert!(matches!(
        generator.generate(),
        Err(IdGenerationError::ClockMovedBackwards {
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
fn test_snowflake_generator_reports_runtime_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout =
        SnowflakeLayout::new(IdMode::Sequential, TimestampPrecision::Second, 7)
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
        Err(IdGenerationError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}

#[test]
fn test_snowflake_generator_rejects_expired_explicit_time() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout =
        SnowflakeLayout::new(IdMode::Sequential, TimestampPrecision::Second, 7)
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
        generator.compose_at(expires_at, 0),
        Err(IdGenerationError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}

#[test]
fn test_snowflake_generator_waits_with_injected_timer() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = Arc::new(
        SnowflakeGenerator::builder(7)
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
    let parts = SnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 11);
    assert_eq!(parts.sequence(), 0);
}

#[test]
fn test_snowflake_generator_preserves_wait_failure_source() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10_250));
    let generator = SnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Second)
        .epoch(epoch)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .wall_clock(time.wall_clock())
        .timer(Arc::new(FaultInjectingTimer::new(
            TimerFailurePoint::Registration,
            || TimeError::InstantOverflow,
        )))
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.generate(),
        Err(IdGenerationError::WaitFailed {
            source: TimeError::InstantOverflow,
        })
    ));
}

mod async_tests {
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
        AsyncIdGenerator,
        DEFAULT_MAX_CLOCK_SKEW,
        GenerationAttempt,
        Id,
        IdGenerationError,
        RestartPolicy,
        SnowflakeGenerator,
        SnowflakeLayout,
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
    ) -> (SnowflakeGenerator, ManualTime) {
        let time = ManualTime::new(now);
        let generator = SnowflakeGenerator::builder(7)
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
    fn test_async_snowflake_generator_convenience_api() {
        let generator = SnowflakeGenerator::new(17)
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
        let parts = SnowflakeLayout::decode(id);
        assert_eq!(parts.host(), 17);
        assert_eq!(parts.timestamp(), 0);
        assert_eq!(parts.sequence(), 3);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_supports_async_trait_object() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let (generator, _time) = build_generator(
            TimestampPrecision::Millisecond,
            epoch,
            epoch + Duration::from_millis(10),
            DEFAULT_MAX_CLOCK_SKEW,
        );
        let generator: Box<
            dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>,
        > = Box::new(generator);

        let _ = generator
            .generate_async()
            .await
            .expect("async trait-object generation should succeed");
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_increments_sequence() {
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

        assert_eq!(SnowflakeLayout::decode(first).sequence(), 0);
        assert_eq!(SnowflakeLayout::decode(second).sequence(), 1);
    }

    #[tokio::test]
    async fn test_unified_qubit_generator_shares_state_across_call_paths() {
        let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10));
        let generator = SnowflakeGenerator::builder(7)
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

        assert_eq!(SnowflakeLayout::decode(first).sequence(), 0);
        assert_eq!(SnowflakeLayout::decode(second).sequence(), 1);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_waits_for_sequence_capacity() {
        let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let (generator, time) = build_generator(
            TimestampPrecision::Millisecond,
            epoch,
            epoch + Duration::from_millis(10),
            DEFAULT_MAX_CLOCK_SKEW,
        );
        let generator = Arc::new(generator);
        for _ in 0..=generator.layout().max_sequence() {
            let _ = generator
                .generate_async()
                .await
                .expect("sequence should remain available");
        }
        let worker_generator = Arc::clone(&generator);
        let worker =
            tokio::spawn(
                async move { worker_generator.generate_async().await },
            );
        let deadline = time.advance_to_next_deadline_async().await;

        assert_eq!(deadline.elapsed_since_origin(), Duration::from_millis(1));
        let id = worker
            .await
            .expect("worker should finish")
            .expect("next time slice should allocate");
        let parts = SnowflakeLayout::decode(id);
        assert_eq!(parts.timestamp(), 11);
        assert_eq!(parts.sequence(), 0);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_wait_is_cancellation_safe() {
        let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let (generator, time) = build_generator(
            TimestampPrecision::Millisecond,
            epoch,
            epoch + Duration::from_millis(10),
            DEFAULT_MAX_CLOCK_SKEW,
        );
        let generator = Arc::new(generator);
        for _ in 0..=generator.layout().max_sequence() {
            let _ = generator
                .generate_async()
                .await
                .expect("sequence should remain available");
        }
        let waiter_observer = time.wait_for_waiters_async(1);
        let worker_generator = Arc::clone(&generator);
        let worker =
            tokio::spawn(
                async move { worker_generator.generate_async().await },
            );
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
        assert_eq!(SnowflakeLayout::decode(id).timestamp(), 11);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_waits_for_small_rollback() {
        let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let (generator, time) = build_generator(
            TimestampPrecision::Millisecond,
            epoch,
            epoch + Duration::from_millis(10),
            Duration::from_millis(1),
        );
        let generator = Arc::new(generator);
        let _ = generator
            .generate_async()
            .await
            .expect("first ID should generate");
        time.reanchor(epoch + Duration::from_millis(9));
        let worker_generator = Arc::clone(&generator);
        let worker =
            tokio::spawn(
                async move { worker_generator.generate_async().await },
            );

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
        assert_eq!(SnowflakeLayout::decode(id).sequence(), 1);
    }

    #[tokio::test]
    async fn test_async_qubit_restart_fence_retries_the_baseline_slice() {
        let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let time = ManualTime::new(epoch + Duration::from_micros(10_250));
        let generator = Arc::new(
            SnowflakeGenerator::builder(7)
                .precision(TimestampPrecision::Millisecond)
                .epoch(epoch)
                .restart_policy(RestartPolicy::WaitNextSlice)
                .wall_clock(time.wall_clock())
                .timer(time.timer())
                .build()
                .expect("configuration should be valid"),
        );
        let worker_generator = Arc::clone(&generator);
        let worker =
            tokio::spawn(
                async move { worker_generator.generate_async().await },
            );

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
        assert_eq!(SnowflakeLayout::decode(id).timestamp(), 11);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_reports_large_rollback() {
        let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
        let (generator, time) = build_generator(
            TimestampPrecision::Millisecond,
            epoch,
            epoch + Duration::from_millis(10),
            Duration::ZERO,
        );
        let _ = generator
            .generate_async()
            .await
            .expect("first ID should generate");
        time.reanchor(epoch + Duration::from_millis(9));

        assert!(matches!(
            generator.generate_async().await,
            Err(IdGenerationError::ClockMovedBackwards { skew, .. })
                if skew == Duration::from_millis(1)
        ));
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_preserves_wait_failure_source() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10_250));
        let generator = SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Second)
            .epoch(epoch)
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
            Err(IdGenerationError::WaitFailed {
                source: TimeError::InstantOverflow,
            })
        ));
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_preserves_completion_failure_source()
     {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10_250));
        let generator = SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Second)
            .epoch(epoch)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .timer(Arc::new(CompletionFailingTimer::new()))
            .build()
            .expect("configuration should be valid");

        assert!(matches!(
            generator.generate_async().await,
            Err(IdGenerationError::WaitFailed {
                source: TimeError::TimerUnavailable {
                    source: TimerUnavailableError::SchedulerWorkerTerminated,
                },
            })
        ));
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_reports_runtime_expiration() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let layout = SnowflakeLayout::new(
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
            Err(IdGenerationError::GeneratorExpired {
                observed_at,
                expires_at: boundary,
            }) if observed_at == expires_at && boundary == expires_at
        ));
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_reports_time_before_epoch() {
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
            Err(IdGenerationError::TimeBeforeEpoch { time: observed, epoch: actual })
                if observed == epoch - Duration::from_secs(1) && actual == epoch
        ));
    }
}
