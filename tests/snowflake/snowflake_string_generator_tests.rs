// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Snowflake decimal-string adapter.

use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    AsyncIdGenerator,
    DEFAULT_MAX_CLOCK_SKEW,
    IdError,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    SnowflakeStringGenerator,
    TimestampPrecision,
};

use crate::support::ManualTime;

#[test]
fn test_snowflake_string_generator_adapts_sync_generator() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let numeric = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let adapter = SnowflakeStringGenerator::new(numeric);
    assert_eq!(adapter.inner().layout().host(), 7);
    let generator: Arc<dyn IdGenerator<String>> = Arc::new(adapter);

    let value = generator.generate().expect("ID should generate");
    let numeric = value.parse::<u64>().expect("ID should be decimal");
    let parts = QubitSnowflakeLayout::decode(numeric);

    assert_eq!(parts.timestamp(), 10);
    assert_eq!(parts.sequence(), 0);
}

#[tokio::test]
async fn test_snowflake_string_generator_adapts_async_generator() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let numeric = QubitSnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build_async()
        .expect("configuration should be valid");
    let generator: Arc<dyn AsyncIdGenerator<String>> =
        Arc::new(SnowflakeStringGenerator::new(numeric));

    let value = generator
        .generate_async()
        .await
        .expect("ID should generate");
    let numeric = value.parse::<u64>().expect("ID should be decimal");

    assert_eq!(QubitSnowflakeLayout::decode(numeric).sequence(), 0);
}

#[test]
fn test_snowflake_string_generator_preserves_generation_error() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = QubitSnowflakeLayout::new(
        IdMode::Sequential,
        TimestampPrecision::Second,
        7,
    )
    .expect("layout should be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("expiration should be representable");
    let time = ManualTime::new(expires_at - Duration::from_nanos(1));
    let numeric = QubitSnowflakeGenerator::builder(7)
        .epoch(epoch)
        .max_clock_skew(DEFAULT_MAX_CLOCK_SKEW)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let generator = SnowflakeStringGenerator::new(numeric);
    time.reanchor(expires_at);

    assert!(matches!(
        generator.generate(),
        Err(IdError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}

#[test]
fn test_snowflake_string_generator_returns_owned_inner_generator() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let numeric = QubitSnowflakeGenerator::builder(7)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    let adapter = SnowflakeStringGenerator::new(numeric);

    assert_eq!(adapter.into_inner().layout().host(), 7);
}
