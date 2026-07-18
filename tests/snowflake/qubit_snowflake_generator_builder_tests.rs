// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Qubit snowflake generator builder.

use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::Arc;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_clock::FixedWallClock;
use qubit_id::{
    IdError,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    TimestampPrecision,
};

use crate::support::ManualTime;

/// Tests that every configurable Qubit generator option is applied.
#[test]
fn test_qubit_snowflake_generator_builder_builds_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = QubitSnowflakeGenerator::builder(17)
        .mode(IdMode::Spread)
        .precision(TimestampPrecision::Millisecond)
        .epoch(epoch)
        .max_clock_skew(Duration::from_millis(37))
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    assert_eq!(generator.layout().mode(), IdMode::Spread);
    assert_eq!(
        generator.layout().precision(),
        TimestampPrecision::Millisecond
    );
    assert_eq!(generator.layout().host(), 17);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.max_clock_skew(), Duration::from_millis(37));

    let id = generator
        .generate()
        .expect("the injected clock should generate an ID");
    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 100);
    assert_eq!(parts.sequence(), 0);
}

/// Tests that builder validation rejects an out-of-range host.
#[test]
fn test_qubit_snowflake_generator_builder_rejects_invalid_host() {
    assert!(matches!(
        QubitSnowflakeGenerator::builder(512).build(),
        Err(IdError::HostOutOfRange {
            host: 512,
            max: 511,
        })
    ));
}

/// Tests the exclusive expiration getter and construction boundary.
#[test]
fn test_qubit_snowflake_generator_builder_enforces_expiration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let layout = QubitSnowflakeLayout::new(
        IdMode::Sequential,
        TimestampPrecision::Second,
        17,
    )
    .expect("Qubit layout must be valid");
    let expires_at = layout
        .expires_at(epoch)
        .expect("Qubit expiration must be representable");
    let last_valid_time = expires_at - Duration::from_nanos(1);

    let generator = QubitSnowflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(Arc::new(FixedWallClock::new(last_valid_time)))
        .build()
        .expect("the instant before expiration must remain valid");
    assert_eq!(generator.expires_at(), expires_at);

    for current_time in [expires_at, expires_at + Duration::from_nanos(1)] {
        let panic = catch_unwind(AssertUnwindSafe(|| {
            QubitSnowflakeGenerator::builder(17)
                .epoch(epoch)
                .wall_clock(Arc::new(FixedWallClock::new(current_time)))
                .build()
                .expect("expired configuration must panic before returning")
        }));

        assert!(
            panic.is_err(),
            "construction at {current_time:?} must panic"
        );
    }
}
