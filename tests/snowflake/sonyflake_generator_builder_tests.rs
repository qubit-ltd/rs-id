// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Sonyflake-style generator builder.

use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_clock::FixedWallClock;
use qubit_id::{
    IdError,
    RestartPolicy,
    SonyflakeGenerator,
    SonyflakeLayout,
};

use crate::support::{
    ManualTime,
    latest_representable_whole_second,
};

/// Tests that every configurable Sonyflake option is applied.
#[test]
fn test_sonyflake_generator_builder_builds_configuration() {
    let start_time = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let time_unit = Duration::from_millis(5);
    let time = ManualTime::new(start_time + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .bits_sequence(7)
        .bits_machine(5)
        .time_unit(time_unit)
        .start_time(start_time)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    assert_eq!(generator.layout().machine_id(), 17);
    assert_eq!(generator.start_time(), start_time);
    assert_eq!(generator.layout().time_unit(), time_unit);
    assert_eq!(generator.layout().bits_time(), 51);
    assert_eq!(generator.layout().bits_sequence(), 7);
    assert_eq!(generator.layout().bits_machine(), 5);

    let id = generator
        .generate()
        .expect("the injected clock should generate an ID");
    let parts = generator.layout().decode(id);
    assert_eq!(parts.elapsed_time(), 20);
    assert_eq!(parts.sequence(), 0);
}

/// Tests the exclusive expiration getter and construction boundary.
#[test]
fn test_sonyflake_generator_builder_enforces_expiration() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time_unit = Duration::from_millis(10);
    let layout = SonyflakeLayout::new(17, 8, 16, time_unit)
        .expect("Sonyflake layout must be valid");
    let expires_at = layout
        .expires_at(start_time)
        .expect("Sonyflake expiration must be representable");
    let last_valid_time = expires_at - Duration::from_nanos(1);

    let generator = SonyflakeGenerator::builder(17)
        .time_unit(time_unit)
        .start_time(start_time)
        .restart_policy(RestartPolicy::Immediate)
        .wall_clock(Arc::new(FixedWallClock::new(last_valid_time)))
        .build()
        .expect("the instant before expiration must remain valid");
    assert_eq!(generator.expires_at(), expires_at);

    for current_time in [expires_at, expires_at + Duration::from_nanos(1)] {
        assert!(
            matches!(
                    SonyflakeGenerator::builder(17)
                        .time_unit(time_unit)
                        .start_time(start_time)
            .restart_policy(RestartPolicy::Immediate)
                        .wall_clock(Arc::new(FixedWallClock::new(current_time)))
                        .build(),
                    Err(IdError::GeneratorExpired {
                        observed_at,
                        expires_at: actual_expiration,
                    }) if observed_at == current_time && actual_expiration == expires_at
                ),
            "construction at {current_time:?} must return GeneratorExpired"
        );
    }
}

#[test]
fn test_sonyflake_generator_builder_rejects_invalid_layout() {
    assert!(matches!(
        SonyflakeGenerator::builder(1_u64 << 16).build(),
        Err(IdError::MachineIdOutOfRange { .. })
    ));
}

#[tokio::test]
async fn test_sonyflake_generator_builder_async_propagates_layout_error() {
    assert!(matches!(
        SonyflakeGenerator::builder(1_u64 << 16).build(),
        Err(IdError::MachineIdOutOfRange { .. })
    ));
}

#[tokio::test]
async fn test_sonyflake_generator_builder_async_propagates_expiration_error() {
    let start_time = latest_representable_whole_second();

    assert!(matches!(
        SonyflakeGenerator::builder(17)
            .start_time(start_time)
            .restart_policy(RestartPolicy::Immediate)
            .build(),
        Err(IdError::ExpirationTimeOverflow { .. })
    ));
}

#[test]
fn test_sonyflake_generator_builder_rejects_future_start_time() {
    let current_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let start_time = current_time + Duration::from_nanos(1);

    assert!(matches!(
        SonyflakeGenerator::builder(17)
            .start_time(start_time)
        .restart_policy(RestartPolicy::Immediate)
            .wall_clock(Arc::new(FixedWallClock::new(current_time)))
            .build(),
        Err(IdError::StartTimeAhead {
            start_time: actual_start,
            current_time: actual_current,
        }) if actual_start == start_time && actual_current == current_time
    ));
}
