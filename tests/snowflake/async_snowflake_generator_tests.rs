// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the asynchronous classic Snowflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    AsyncIdGenerator,
    AsyncSnowflakeGenerator,
    IdError,
    RestartPolicy,
    SnowflakeGenerator,
    SnowflakeLayout,
};

use crate::support::ManualTime;

#[test]
fn test_async_snowflake_generator_convenience_api() {
    let generator = AsyncSnowflakeGenerator::new(17)
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
    let generator: Arc<dyn AsyncIdGenerator<u64>> = Arc::new(
        SnowflakeGenerator::builder(17)
            .epoch(epoch)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build_async()
            .expect("configuration should be valid"),
    );

    assert!(generator.generate_async().await.is_ok());
}

#[tokio::test]
async fn test_async_snowflake_generator_increments_sequence() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator: AsyncSnowflakeGenerator = SnowflakeGenerator::builder(17)
        .epoch(epoch)
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

    assert_eq!(SnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(SnowflakeLayout::decode(second).sequence(), 1);
}

#[tokio::test]
async fn test_async_snowflake_generator_waits_with_injected_timer() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = Arc::new(
        SnowflakeGenerator::builder(17)
            .epoch(epoch)
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
        Duration::from_micros(750)
    );
    let id = worker
        .await
        .expect("worker should finish")
        .expect("next millisecond should allocate");
    assert_eq!(SnowflakeLayout::decode(id).timestamp(), 11);
}

#[tokio::test]
async fn test_async_snowflake_generator_reports_runtime_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = SnowflakeLayout::new(17).expect("layout should be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("expiration should be representable");
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let generator = SnowflakeGenerator::builder(17)
        .epoch(epoch)
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
