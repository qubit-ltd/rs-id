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
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    IdError,
    IdGenerator,
    RestartPolicy,
    SnowflakeGenerator,
    SnowflakeLayout,
};

use crate::support::ManualTime;

#[test]
fn test_snowflake_generator_new_uses_defaults() {
    let generator = SnowflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().node_id(), 17);
}

#[test]
fn test_snowflake_generator_supports_sync_trait_object() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator: Arc<dyn IdGenerator<u64>> = Arc::new(
        SnowflakeGenerator::builder(17)
            .epoch(epoch)
            .wall_clock(time.wall_clock())
            .timer(time.timer())
            .build()
            .expect("configuration should be valid"),
    );

    assert!(generator.generate().is_ok());
}

mod inherent_api_tests {
    use super::ManualTime;
    use qubit_id::SnowflakeGenerator;
    use std::time::{
        Duration,
        UNIX_EPOCH,
    };

    #[test]
    fn test_snowflake_generator_supports_inherent_generate() {
        let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(epoch + Duration::from_millis(10));
        let generator = SnowflakeGenerator::builder(7)
            .epoch(epoch)
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
fn test_snowflake_generator_increments_sequence_in_same_millisecond() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SnowflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let first = generator.generate().expect("first ID should generate");
    let second = generator.generate().expect("second ID should generate");

    assert_eq!(SnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(SnowflakeLayout::decode(second).sequence(), 1);
}

#[test]
fn test_snowflake_generator_waits_with_injected_timer() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_micros(10_250));
    let generator = Arc::new(
        SnowflakeGenerator::builder(17)
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

    assert_eq!(SnowflakeLayout::decode(id).timestamp(), 11);
    assert_eq!(SnowflakeLayout::decode(id).sequence(), 0);
}

#[test]
fn test_snowflake_generator_reports_clock_rollback() {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(epoch + Duration::from_millis(10));
    let generator = SnowflakeGenerator::builder(17)
        .epoch(epoch)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    generator.generate().expect("first ID should generate");
    time.reanchor(epoch + Duration::from_millis(9));

    assert!(matches!(
        generator.generate(),
        Err(IdError::ClockMovedBackwards { skew, .. })
            if skew == Duration::from_millis(1)
    ));
}

#[test]
fn test_snowflake_generator_reports_runtime_expiration() {
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
        .build()
        .expect("configuration should be valid");
    time.reanchor(expires_at);

    assert!(matches!(
        generator.generate(),
        Err(IdError::GeneratorExpired {
            observed_at,
            expires_at: boundary,
        }) if observed_at == expires_at && boundary == expires_at
    ));
}
