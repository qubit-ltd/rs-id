// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Sonyflake-style generator builder.

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
    IdGenerator,
    SonyflakeGenerator,
    SonyflakeLayout,
};

use crate::support::ManualTime;

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
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(generator.layout().machine_id(), 17);
    assert_eq!(generator.start_time(), start_time);
    assert_eq!(generator.layout().time_unit(), time_unit);
    assert_eq!(generator.layout().bits_time(), 51);
    assert_eq!(generator.layout().bits_sequence(), 7);
    assert_eq!(generator.layout().bits_machine(), 5);

    let id = generator
        .next_id()
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
        .wall_clock(Arc::new(FixedWallClock::new(last_valid_time)))
        .build()
        .expect("the instant before expiration must remain valid");
    assert_eq!(generator.expires_at(), expires_at);

    for current_time in [expires_at, expires_at + Duration::from_nanos(1)] {
        let panic = catch_unwind(AssertUnwindSafe(|| {
            SonyflakeGenerator::builder(17)
                .time_unit(time_unit)
                .start_time(start_time)
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
