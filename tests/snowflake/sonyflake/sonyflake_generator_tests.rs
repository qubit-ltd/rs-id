// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the synchronous Sonyflake generator.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use qubit_id::GenerationAttempt;
use qubit_id::Id;
use qubit_id::IdGenerationError;
use qubit_id::IdGenerator;
use qubit_id::RestartPolicy;
use qubit_id::SonyflakeGenerator;
use qubit_id::SonyflakeLayout;
use qubit_id::TryIdGenerator;

use crate::support::ManualTime;

#[test]
fn test_sonyflake_generator_new_uses_defaults() {
    let generator = SonyflakeGenerator::new(17).expect("default configuration should be valid");

    assert_eq!(generator.layout().machine_id(), 17);
}

#[test]
fn test_sonyflake_generator_compose_at_matches_layout_parts() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let id = generator
        .compose_at(epoch + Duration::from_millis(450), 9)
        .expect("elapsed time and sequence should be valid");
    let parts = generator.layout().decode(id);

    assert_eq!(parts.elapsed_time(), 45);
    assert_eq!(parts.sequence(), 9);
    assert_eq!(parts.machine_id(), 17);
}

#[test]
fn test_sonyflake_generator_compose_at_rejects_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let before_epoch = epoch - Duration::from_nanos(1);

    assert!(matches!(
        generator.compose_at(before_epoch, 0),
        Err(IdGenerationError::TimeBeforeEpoch {
            time: actual_time,
            epoch: actual_epoch,
        }) if actual_time == before_epoch && actual_epoch == epoch
    ));
}

#[test]
fn test_sonyflake_generator_supports_sync_trait_object() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator: Arc<dyn IdGenerator<Id>> = Arc::new(
        SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid"),
    );

    assert!(generator.generate().is_ok());
}

#[test]
fn test_sonyflake_generator_supports_nonblocking_trait_object_and_inherent_api() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    assert!(matches!(generator.try_generate(), Ok(GenerationAttempt::Generated(_))));

    let generator: Arc<dyn TryIdGenerator<Id>> = Arc::new(generator);
    assert!(matches!(generator.try_generate(), Ok(GenerationAttempt::Generated(_))));
}

mod inherent_api_tests {
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use qubit_id::RestartPolicy;
    use qubit_id::SonyflakeGenerator;

    use super::ManualTime;

    #[test]
    fn test_sonyflake_generator_supports_inherent_generate() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(100));
        let generator = SonyflakeGenerator::builder(7)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid");

        let _id = generator.generate().expect("inherent generation should succeed");
    }
}

#[test]
fn test_sonyflake_generator_increments_sequence_in_same_time_unit() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let first = generator.generate().expect("first ID should generate");
    let second = generator.generate().expect("second ID should generate");

    assert_eq!(generator.layout().decode(first).sequence(), 0);
    assert_eq!(generator.layout().decode(second).sequence(), 1);
}

#[tokio::test]
async fn test_sonyflake_clone_shares_allocation_state() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let first = generator.generate().expect("first allocation");
    let cloned = generator.clone();
    let second = cloned.generate().expect("allocation through clone");
    let third = match generator.try_generate().expect("try allocation") {
        GenerationAttempt::Generated(id) => id,
        GenerationAttempt::RetryAfter { .. } => panic!("fixed slice has capacity"),
    };
    let fourth = crate::support::complete_with_timeout("allocation through clone", cloned.generate_async())
        .await
        .expect("async allocation");
    let parts = [first, second, third, fourth].map(|id| generator.layout().decode(id));

    assert_eq!(parts.map(|part| part.elapsed_time()), [10; 4]);
    assert_eq!(parts.map(|part| part.sequence()), [0, 1, 2, 3]);
    assert_eq!(parts.map(|part| part.machine_id()), [17; 4]);
    assert_eq!(HashSet::from([first, second, third, fourth]).len(), 4);
}

#[test]
fn test_sonyflake_generator_waits_with_injected_timer() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(105));
    let generator = Arc::new(
        SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid"),
    );
    let worker_generator = Arc::clone(&generator);
    let worker = crate::support::spawn_with_timeout("sonyflake injected timer allocation", move || {
        worker_generator.generate()
    });

    time.advance_to_next_deadline();
    let id = worker.finish().expect("next time unit should allocate");

    assert_eq!(generator.layout().decode(id).elapsed_time(), 11);
    assert_eq!(generator.layout().decode(id).sequence(), 0);
}

#[test]
fn test_sonyflake_generator_reports_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let _ = generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(90));

    assert!(matches!(
        generator.generate(),
        Err(IdGenerationError::ClockMovedBackwards { skew, .. })
            if skew == Duration::from_millis(10)
    ));
}

#[test]
fn test_sonyflake_generator_retries_configured_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(20));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .max_clock_skew(Duration::from_millis(10))
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let _ = generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(10));

    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::RetryAfter { delay }) if delay == Duration::from_millis(10)
    ));

    time.reanchor(epoch + Duration::from_millis(20));
    assert!(matches!(generator.try_generate(), Ok(GenerationAttempt::Generated(_))));
}

#[test]
fn test_sonyflake_generator_reports_runtime_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = SonyflakeLayout::new(17, 8, 16, Duration::from_millis(10)).expect("layout should be valid");
    let expires_at = layout.expires_at(epoch).expect("expiration should be representable");
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
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
fn test_sonyflake_nanosecond_time_unit_quantizes_elapsed_time() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_nanos(123);
    let unit = Duration::from_nanos(1_500_000);
    let time = ManualTime::new(epoch + Duration::from_nanos(3_000_000));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .time_unit(unit)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("nanosecond unit is valid");

    for (offset, expected) in [(2_999_999, 1), (3_000_000, 2), (3_000_001, 2)] {
        let id = generator
            .compose_at(epoch + Duration::from_nanos(offset), 3)
            .expect("elapsed time and sequence should be valid");
        let parts = generator.layout().decode(id);

        assert_eq!(parts.elapsed_time(), expected);
        assert_eq!(parts.sequence(), 3);
        assert_eq!(parts.machine_id(), 17);
    }
}

#[test]
fn test_sonyflake_nanosecond_time_unit_retry_delay_precision() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_nanos(123);
    let unit = Duration::from_nanos(1_500_000);

    for (offset, delay_ns, next_slice) in [(2_999_999, 1, 2), (3_000_000, 1_500_000, 3), (3_000_001, 1_499_999, 3)] {
        let now = epoch + Duration::from_nanos(offset);
        let time = ManualTime::new(now);
        let generator = SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .time_unit(unit)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("nanosecond unit is valid");
        let delay = Duration::from_nanos(delay_ns);

        assert_eq!(
            generator.try_generate().expect("first attempt should retry"),
            GenerationAttempt::RetryAfter { delay }
        );
        time.reanchor(now + delay);
        let id = match generator.try_generate().expect("boundary attempt should succeed") {
            GenerationAttempt::Generated(id) => id,
            other => panic!("boundary should permit generation: {other:?}"),
        };
        let parts = generator.layout().decode(id);

        assert_eq!(parts.elapsed_time(), next_slice);
        assert_eq!(parts.sequence(), 0);
        assert_eq!(parts.machine_id(), 17);
    }
}

#[test]
fn test_sonyflake_nanosecond_time_unit_expiration_and_builder_boundaries() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000) + Duration::from_nanos(123);
    let unit = Duration::from_nanos(1_500_000);
    let lifetime = unit
        .checked_mul(1 << 30)
        .expect("lifetime multiplication should fit")
        .checked_mul(1 << 9)
        .expect("lifetime multiplication should fit");
    let expires_at = epoch.checked_add(lifetime).expect("expiration should fit");
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let generator = SonyflakeGenerator::builder(17)
        .epoch(epoch)
        .time_unit(unit)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("generator should be valid immediately before expiration");

    assert_eq!(generator.expires_at(), expires_at);
    let last_valid = generator
        .compose_at(expires_at - Duration::from_nanos(1), 0)
        .expect("time immediately before expiration should be valid");
    assert_eq!(generator.layout().decode(last_valid).elapsed_time(), (1_u64 << 39) - 1);
    let generated = match generator
        .try_generate()
        .expect("time immediately before expiration should generate")
    {
        GenerationAttempt::Generated(id) => id,
        other => panic!("time immediately before expiration should generate: {other:?}"),
    };
    assert_eq!(generator.layout().decode(generated).elapsed_time(), (1_u64 << 39) - 1);

    for observed_at in [expires_at, expires_at + Duration::from_nanos(1)] {
        time.reanchor(observed_at);
        assert!(matches!(
            generator.compose_at(observed_at, 0),
            Err(IdGenerationError::GeneratorExpired { observed_at: actual, expires_at: boundary })
                if actual == observed_at && boundary == expires_at
        ));
        assert!(matches!(
            generator.try_generate(),
            Err(IdGenerationError::GeneratorExpired { observed_at: actual, expires_at: boundary })
                if actual == observed_at && boundary == expires_at
        ));
    }

    for (observed_at, should_build) in [
        (expires_at - Duration::from_nanos(1), true),
        (expires_at, false),
        (expires_at + Duration::from_nanos(1), false),
    ] {
        let build_time = ManualTime::new(observed_at);
        let result = SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .time_unit(unit)
            .wall_clock(build_time.wall_clock())
            .timer(build_time.timer())
            .build();

        if should_build {
            assert!(result.is_ok(), "builder should accept {observed_at:?}");
        } else {
            assert!(matches!(
                result,
                Err(IdGenerationError::GeneratorExpired { observed_at: actual, expires_at: boundary })
                    if actual == observed_at && boundary == expires_at
            ));
        }
    }
}

mod async_tests {
    // =============================================================================
    //    Copyright (c) 2026 Haixing Hu.
    //
    //    SPDX-License-Identifier: Apache-2.0
    //
    //    Licensed under the Apache License, Version 2.0.
    // =============================================================================
    //! Integration tests for the asynchronous Sonyflake generator.

    use std::sync::Arc;
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use qubit_id::AsyncIdGenerator;
    use qubit_id::Id;
    use qubit_id::IdGenerationError;
    use qubit_id::RestartPolicy;
    use qubit_id::SonyflakeGenerator;
    use qubit_id::SonyflakeLayout;

    use crate::support::ManualTime;

    #[test]
    fn test_async_sonyflake_generator_convenience_api() {
        let generator = SonyflakeGenerator::new(17).expect("default configuration should be valid");

        assert_eq!(generator.layout().machine_id(), 17);
        assert_eq!(
            generator.expires_at(),
            generator
                .layout()
                .expires_at(generator.epoch())
                .expect("default expiration should be representable")
        );
    }

    #[tokio::test]
    async fn test_async_sonyflake_generator_supports_async_trait_object() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(100));
        let generator: Arc<dyn AsyncIdGenerator<Id>> = Arc::new(
            SonyflakeGenerator::builder(17)
                .epoch(epoch)
                .restart_policy(RestartPolicy::Immediate)
                .wall_clock(time.wall_clock())
                .timer(time.timer())
                .build()
                .expect("configuration should be valid"),
        );

        assert!(generator.generate_async().await.is_ok());
    }

    #[tokio::test]
    async fn test_async_sonyflake_generator_increments_sequence() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(100));
        let generator: SonyflakeGenerator = SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid");

        let first = generator.generate_async().await.expect("first ID should generate");
        let second = generator.generate_async().await.expect("second ID should generate");

        assert_eq!(generator.layout().decode(first).sequence(), 0);
        assert_eq!(generator.layout().decode(second).sequence(), 1);
    }

    #[tokio::test]
    async fn test_async_sonyflake_generator_waits_with_injected_timer() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(105));
        let generator = Arc::new(
            SonyflakeGenerator::builder(17)
                .epoch(epoch)
                .restart_policy(RestartPolicy::WaitNextSlice)
                .wall_clock(time.wall_clock())
                .timer(time.timer())
                .build()
                .expect("configuration should be valid"),
        );
        let worker_generator = Arc::clone(&generator);
        let mut worker = tokio::spawn(async move { worker_generator.generate_async().await });

        assert_eq!(
            time.advance_to_next_deadline_async().await.elapsed_since_origin(),
            Duration::from_millis(5)
        );
        let joined = match tokio::time::timeout(Duration::from_secs(10), &mut worker).await {
            Ok(joined) => joined,
            Err(_) => {
                worker.abort();
                panic!("test coordination timed out: generator worker completion");
            }
        };
        let id = joined
            .expect("worker should finish")
            .expect("next time unit should allocate");
        assert_eq!(generator.layout().decode(id).elapsed_time(), 11);
    }

    #[tokio::test]
    async fn test_async_sonyflake_generator_reports_runtime_expiration() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let layout = SonyflakeLayout::new(17, 8, 16, Duration::from_millis(10)).expect("layout should be valid");
        let expires_at = layout.expires_at(epoch).expect("expiration should be representable");
        let time = ManualTime::new(expires_at - Duration::from_nanos(1));
        let generator = SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid");
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
    async fn test_async_sonyflake_generator_reports_time_before_epoch() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10));
        let generator = SonyflakeGenerator::builder(17)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid");
        time.reanchor(epoch - Duration::from_millis(1));

        assert!(matches!(
            generator.generate_async().await,
            Err(IdGenerationError::TimeBeforeEpoch { time: observed, epoch: actual })
                if observed == epoch - Duration::from_millis(1)
                    && actual == epoch
        ));
    }
}
