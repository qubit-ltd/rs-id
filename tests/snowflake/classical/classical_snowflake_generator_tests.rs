// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the synchronous classic Snowflake generator.

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use qubit_id::ClassicalSnowflakeGenerator;
use qubit_id::ClassicalSnowflakeLayout;
use qubit_id::GenerationAttempt;
use qubit_id::Id;
use qubit_id::IdGenerationError;
use qubit_id::IdGenerator;
use qubit_id::RestartPolicy;
use qubit_id::TryIdGenerator;

use crate::support::ManualTime;

#[test]
fn test_classical_snowflake_generator_new_uses_defaults() {
    let generator = ClassicalSnowflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().node_id(), 17);
}

#[test]
fn test_classical_snowflake_generator_compose_at_matches_layout_parts() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let id = generator
        .compose_at(epoch + Duration::from_millis(45), 9)
        .expect("timestamp and sequence should be valid");
    let parts = ClassicalSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 45);
    assert_eq!(parts.sequence(), 9);
    assert_eq!(parts.node_id(), 17);
}

#[test]
fn test_classical_snowflake_generator_compose_at_rejects_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = ClassicalSnowflakeGenerator::builder(17)
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
fn test_classical_snowflake_generator_supports_sync_trait_object() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator: Arc<dyn IdGenerator<Id>> = Arc::new(
        ClassicalSnowflakeGenerator::builder(17)
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
fn test_classical_snowflake_generator_supports_nonblocking_trait_object_and_inherent_api()
 {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(_))
    ));

    let generator: Arc<dyn TryIdGenerator<Id>> = Arc::new(generator);
    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(_))
    ));
}

mod inherent_api_tests {
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use qubit_id::ClassicalSnowflakeGenerator;
    use qubit_id::RestartPolicy;

    use super::ManualTime;

    #[test]
    fn test_classical_snowflake_generator_supports_inherent_generate() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10));
        let generator = ClassicalSnowflakeGenerator::builder(7)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid");

        let _id = generator
            .generate()
            .expect("inherent generation should succeed");
    }
}

#[test]
fn test_classical_snowflake_generator_increments_sequence_in_same_millisecond()
{
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let first = generator.generate().expect("first ID should generate");
    let second = generator.generate().expect("second ID should generate");

    assert_eq!(ClassicalSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(ClassicalSnowflakeLayout::decode(second).sequence(), 1);
}

#[test]
fn test_classical_snowflake_generator_waits_with_injected_timer() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = Arc::new(
        ClassicalSnowflakeGenerator::builder(17)
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
        .expect("next millisecond should allocate");

    assert_eq!(ClassicalSnowflakeLayout::decode(id).timestamp(), 11);
    assert_eq!(ClassicalSnowflakeLayout::decode(id).sequence(), 0);
}

#[test]
fn test_classical_snowflake_generator_reports_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let _ = generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));

    assert!(matches!(
        generator.generate(),
        Err(IdGenerationError::ClockMovedBackwards { skew, .. })
            if skew == Duration::from_millis(1)
    ));
}

#[test]
fn test_classical_snowflake_generator_retries_configured_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .max_clock_skew(Duration::from_millis(1))
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let _ = generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));

    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::RetryAfter { delay }) if delay == Duration::from_millis(1)
    ));

    time.reanchor(epoch + Duration::from_millis(10));
    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(_))
    ));
}

#[test]
fn test_classical_snowflake_generator_reports_runtime_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout =
        ClassicalSnowflakeLayout::new(17).expect("layout should be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("expiration should be representable");
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let generator = ClassicalSnowflakeGenerator::builder(17)
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

mod async_tests {
    // =============================================================================
    //    Copyright (c) 2026 Haixing Hu.
    //
    //    SPDX-License-Identifier: Apache-2.0
    //
    //    Licensed under the Apache License, Version 2.0.
    // =============================================================================
    //! Integration tests for the asynchronous classic Snowflake generator.

    use std::sync::Arc;
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    use qubit_id::AsyncIdGenerator;
    use qubit_id::ClassicalSnowflakeGenerator;
    use qubit_id::ClassicalSnowflakeLayout;
    use qubit_id::Id;
    use qubit_id::IdGenerationError;
    use qubit_id::RestartPolicy;

    use crate::support::ManualTime;

    #[test]
    fn test_async_snowflake_generator_convenience_api() {
        let generator = ClassicalSnowflakeGenerator::new(17)
            .expect("default configuration should be valid");

        assert_eq!(generator.layout().node_id(), 17);
        assert_eq!(
            generator.expires_at(),
            generator
                .layout()
                .expires_at(generator.epoch())
                .expect("default expiration should be representable")
        );
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_supports_async_trait_object() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10));
        let generator: Arc<dyn AsyncIdGenerator<Id>> = Arc::new(
            ClassicalSnowflakeGenerator::builder(17)
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
    async fn test_async_snowflake_generator_increments_sequence() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10));
        let generator: ClassicalSnowflakeGenerator =
            ClassicalSnowflakeGenerator::builder(17)
                .epoch(epoch)
                .restart_policy(RestartPolicy::Immediate)
                .wall_clock(time.wall_clock())
                .timer(time.timer())
                .build()
                .expect("configuration should be valid");

        let first = generator
            .generate_async()
            .await
            .expect("first ID should generate");
        let second = generator
            .generate_async()
            .await
            .expect("second ID should generate");

        assert_eq!(ClassicalSnowflakeLayout::decode(first).sequence(), 0);
        assert_eq!(ClassicalSnowflakeLayout::decode(second).sequence(), 1);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_waits_with_injected_timer() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_micros(10_250));
        let generator = Arc::new(
            ClassicalSnowflakeGenerator::builder(17)
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
            time.advance_to_next_deadline_async()
                .await
                .elapsed_since_origin(),
            Duration::from_micros(750)
        );
        let id = worker
            .await
            .expect("worker should finish")
            .expect("next millisecond should allocate");
        assert_eq!(ClassicalSnowflakeLayout::decode(id).timestamp(), 11);
    }

    #[tokio::test]
    async fn test_async_snowflake_generator_reports_runtime_expiration() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let layout =
            ClassicalSnowflakeLayout::new(17).expect("layout should be valid");
        let expires_at = layout
            .expires_at(epoch)
            .expect("expiration should be representable");
        let time = ManualTime::new(expires_at - Duration::from_nanos(1));
        let generator = ClassicalSnowflakeGenerator::builder(17)
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
    async fn test_async_snowflake_generator_reports_time_before_epoch() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(1));
        let generator = ClassicalSnowflakeGenerator::builder(17)
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
                if observed == epoch - Duration::from_millis(1) && actual == epoch
        ));
    }
}
