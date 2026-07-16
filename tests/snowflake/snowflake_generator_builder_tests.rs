// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the classic Snowflake generator builder.

use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    IdError,
    IdGenerator,
    SnowflakeGenerator,
};

use crate::support::ManualTime;

#[test]
fn test_snowflake_generator_builder_builds_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(100));
    let generator = SnowflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .blocking_sleeper(time.blocking_sleeper())
        .build()
        .expect("configuration should be valid");

    assert_eq!(generator.node_id(), 17);
    assert_eq!(generator.epoch(), epoch);
    let id = generator
        .next_id()
        .expect("default immediate policy should allocate");
    assert_eq!(generator.extract_timestamp(id), 100);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_snowflake_generator_builder_rejects_invalid_node() {
    assert!(matches!(
        SnowflakeGenerator::builder(1_024).build(),
        Err(IdError::NodeOutOfRange {
            node_id: 1_024,
            max: 1_023,
        })
    ));
}
