// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Sonyflake-style generator builder.

use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    IdGenerator,
    SonyflakeGenerator,
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

    assert_eq!(generator.machine_id(), 17);
    assert_eq!(generator.start_time(), start_time);
    assert_eq!(generator.time_unit(), time_unit);
    assert_eq!(generator.bits_time(), 51);
    assert_eq!(generator.bits_sequence(), 7);
    assert_eq!(generator.bits_machine(), 5);

    let id = generator
        .next_id()
        .expect("the injected clock should generate an ID");
    assert_eq!(generator.extract_elapsed_time(id), 20);
    assert_eq!(generator.extract_sequence(id), 0);
}
