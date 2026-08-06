// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the classic Snowflake generator builder.

use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_clock::FixedWallClock;
use qubit_id::{
    ClassicalSnowflakeGenerator,
    ClassicalSnowflakeLayout,
    IdGenerationError,
    RestartPolicy,
};

use crate::support::{
    ManualTime,
    latest_representable_whole_second,
};

#[test]
fn test_classical_snowflake_generator_builder_builds_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .max_clock_skew(Duration::from_millis(37))
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    assert_eq!(generator.layout().node_id(), 17);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.max_clock_skew(), Duration::from_millis(37));
    let id = generator
        .generate()
        .expect("default immediate policy should allocate");
    let parts = ClassicalSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 100);
    assert_eq!(parts.sequence(), 0);
}

/// Verifies that an unconfigured builder allocates in its first observed slice.
#[test]
fn test_classical_snowflake_generator_builder_defaults_to_immediate_allocation()
{
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("default configuration should be valid");

    assert_eq!(generator.max_clock_skew(), Duration::ZERO);

    assert!(matches!(
        generator.try_generate(),
        Ok(qubit_id::GenerationAttempt::Generated(_))
    ));
}

#[test]
fn test_classical_snowflake_generator_builder_rejects_invalid_node() {
    assert!(matches!(
        ClassicalSnowflakeGenerator::builder(1_024).build(),
        Err(IdGenerationError::NodeOutOfRange {
            node_id: 1_024,
            max: 1_023,
        })
    ));
}

#[tokio::test]
async fn test_classical_snowflake_generator_builder_async_propagates_layout_error()
 {
    assert!(matches!(
        ClassicalSnowflakeGenerator::builder(1_024).build(),
        Err(IdGenerationError::NodeOutOfRange {
            node_id: 1_024,
            max: 1_023,
        })
    ));
}

#[tokio::test]
async fn test_classical_snowflake_generator_builder_async_propagates_expiration_error()
 {
    let epoch = latest_representable_whole_second();

    assert!(matches!(
        ClassicalSnowflakeGenerator::builder(17)
            .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
            .wall_clock(Arc::new(FixedWallClock::new(epoch)))
            .build(),
        Err(IdGenerationError::ExpirationTimeOverflow { .. })
    ));
}

#[test]
fn test_classical_snowflake_generator_builder_rejects_extreme_future_epoch_before_expiration_overflow()
 {
    let epoch = latest_representable_whole_second();
    let current_time = epoch - Duration::from_nanos(1);

    assert!(matches!(
        ClassicalSnowflakeGenerator::builder(17)
            .epoch(epoch)
            .wall_clock(Arc::new(FixedWallClock::new(current_time)))
            .build(),
        Err(IdGenerationError::EpochAhead {
            epoch: actual_epoch,
            current_time: actual_current,
        }) if actual_epoch == epoch && actual_current == current_time
    ));
}

/// Tests that builder validation rejects an epoch later than its wall clock.
#[test]
fn test_classical_snowflake_generator_builder_rejects_future_epoch() {
    let current_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let epoch = current_time + Duration::from_nanos(1);

    assert!(matches!(
        ClassicalSnowflakeGenerator::builder(17)
            .epoch(epoch)
            .wall_clock(Arc::new(FixedWallClock::new(current_time)))
            .build(),
        Err(IdGenerationError::EpochAhead {
            epoch: actual_epoch,
            current_time: actual_current,
        }) if actual_epoch == epoch && actual_current == current_time
    ));
}

#[test]
fn test_classical_snowflake_generator_builder_enforces_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = ClassicalSnowflakeLayout::new(17)
        .expect("classic Snowflake layout must be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("classic expiration must be representable");
    let last_valid_time = expires_at - Duration::from_nanos(1);

    let generator = ClassicalSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(Arc::new(FixedWallClock::new(last_valid_time)))
        .build()
        .expect("the instant before expiration must remain valid");
    assert_eq!(generator.expires_at(), expires_at);

    for current_time in [expires_at, expires_at + Duration::from_nanos(1)] {
        assert!(
            matches!(
                    ClassicalSnowflakeGenerator::builder(17)
                        .epoch(epoch)
            .restart_policy(RestartPolicy::Immediate)
                        .wall_clock(Arc::new(FixedWallClock::new(current_time)))
                        .build(),
                    Err(IdGenerationError::GeneratorExpired {
                        observed_at,
                        expires_at: actual_expiration,
                    }) if observed_at == current_time && actual_expiration == expires_at
                ),
            "construction at {current_time:?} must return GeneratorExpired"
        );
    }
}
