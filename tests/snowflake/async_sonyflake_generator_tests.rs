// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the asynchronous Sonyflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    AsyncIdGenerator,
    AsyncSonyflakeGenerator,
    IdError,
    RestartPolicy,
    SonyflakeGenerator,
    SonyflakeLayout,
};

use crate::support::ManualTime;

#[test]
fn test_async_sonyflake_generator_convenience_api() {
    let generator = AsyncSonyflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().machine_id(), 17);
    assert_eq!(
        generator.expires_at(),
        generator
            .layout()
            .expires_at(generator.start_time())
            .expect("default expiration should be representable")
    );
}

#[tokio::test]
async fn test_async_sonyflake_generator_supports_async_trait_object() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(100));
    let generator: Arc<dyn AsyncIdGenerator<u64>> = Arc::new(
        SonyflakeGenerator::builder(17)
            .start_time(start_time)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build_async()
            .expect("configuration should be valid"),
    );

    assert!(generator.generate_async().await.is_ok());
}

#[tokio::test]
async fn test_async_sonyflake_generator_increments_sequence() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(100));
    let generator: AsyncSonyflakeGenerator = SonyflakeGenerator::builder(17)
        .start_time(start_time)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build_async()
        .expect("configuration should be valid");

    let first = generator
        .generate_async()
        .await
        .expect("first ID should generate");
    let second = generator
        .generate_async()
        .await
        .expect("second ID should generate");

    assert_eq!(generator.layout().decode(first).sequence(), 0);
    assert_eq!(generator.layout().decode(second).sequence(), 1);
}

#[tokio::test]
async fn test_async_sonyflake_generator_waits_with_injected_timer() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(105));
    let generator = Arc::new(
        SonyflakeGenerator::builder(17)
            .start_time(start_time)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build_async()
            .expect("configuration should be valid"),
    );
    let worker_generator = Arc::clone(&generator);
    let worker =
        tokio::spawn(async move { worker_generator.generate_async().await });

    assert_eq!(
        time.advance_to_next_deadline_async()
            .await
            .elapsed_since_origin(),
        Duration::from_millis(5)
    );
    let id = worker
        .await
        .expect("worker should finish")
        .expect("next time unit should allocate");
    assert_eq!(generator.layout().decode(id).elapsed_time(), 11);
}

#[tokio::test]
async fn test_async_sonyflake_generator_reports_runtime_expiration() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = SonyflakeLayout::new(17, 8, 16, Duration::from_millis(10))
        .expect("layout should be valid");
    let expires_at = layout
        .expires_at(start_time)
        .expect("expiration should be representable");
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let generator = SonyflakeGenerator::builder(17)
        .start_time(start_time)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build_async()
        .expect("configuration should be valid");
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
async fn test_async_sonyflake_generator_reports_time_before_start() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(10));
    let generator = SonyflakeGenerator::builder(17)
        .start_time(start_time)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build_async()
        .expect("configuration should be valid");
    time.reanchor(start_time - Duration::from_millis(1));

    assert!(matches!(
        generator.generate_async().await,
        Err(IdError::TimeBeforeEpoch { time: observed, epoch: actual })
            if observed == start_time - Duration::from_millis(1)
                && actual == start_time
    ));
}
