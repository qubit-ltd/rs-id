// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Integration tests for the synchronous Sonyflake generator.

use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::{
    IdError,
    RestartPolicy,
    SonyflakeGenerator,
    SonyflakeLayout,
};

use crate::support::ManualTime;

#[test]
fn test_sonyflake_generator_new_uses_defaults() {
    let generator = SonyflakeGenerator::new(17)
        .expect("default configuration should be valid");

    assert_eq!(generator.layout().machine_id(), 17);
}

mod inherent_api_tests {
    use super::ManualTime;
    use qubit_id::SonyflakeGenerator;
    use std::time::{
        Duration,
        UNIX_EPOCH,
    };

    #[test]
    fn test_sonyflake_generator_supports_inherent_generate() {
        let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let time = ManualTime::new(start_time + Duration::from_millis(100));
        let generator = SonyflakeGenerator::builder(7)
            .start_time(start_time)
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
fn test_sonyflake_generator_increments_sequence_in_same_time_unit() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .start_time(start_time)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");

    let first = generator.generate().expect("first ID should generate");
    let second = generator.generate().expect("second ID should generate");

    assert_eq!(generator.layout().decode(first).sequence(), 0);
    assert_eq!(generator.layout().decode(second).sequence(), 1);
}

#[test]
fn test_sonyflake_generator_waits_with_injected_timer() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(105));
    let generator = Arc::new(
        SonyflakeGenerator::builder(17)
            .start_time(start_time)
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
        .expect("next time unit should allocate");

    assert_eq!(generator.layout().decode(id).elapsed_time(), 11);
    assert_eq!(generator.layout().decode(id).sequence(), 0);
}

#[test]
fn test_sonyflake_generator_reports_clock_rollback() {
    let start_time = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let time = ManualTime::new(start_time + Duration::from_millis(100));
    let generator = SonyflakeGenerator::builder(17)
        .start_time(start_time)
        .wall_clock(time.wall_clock())
        .timer(time.timer())
        .build()
        .expect("configuration should be valid");
    generator.generate().expect("first ID should generate");
    time.reanchor(start_time + Duration::from_millis(90));

    assert!(matches!(
        generator.generate(),
        Err(IdError::ClockMovedBackwards { skew, .. })
            if skew == Duration::from_millis(10)
    ));
}

#[test]
fn test_sonyflake_generator_reports_runtime_expiration() {
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
