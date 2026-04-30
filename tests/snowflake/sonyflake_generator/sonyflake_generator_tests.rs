/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for the Sonyflake-style generator.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use qubit_id::{IdError, IdGenerator, SonyflakeGenerator};

#[test]
fn test_sonyflake_generator_default_layout_matches_sonyflake() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        0x1234,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        move || epoch + Duration::from_millis(120),
    )
    .expect("configuration should be valid");

    let id = generator
        .compose(12, 7, 0x1234)
        .expect("parts should be valid");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
    assert_eq!(id, (12_u64 << 24) | (7_u64 << 16) | 0x1234);
    assert_eq!(generator.extract_elapsed_time(id), 12);
    assert_eq!(generator.extract_sequence(id), 7);
    assert_eq!(generator.extract_machine_id(id), 0x1234);
}

#[test]
fn test_sonyflake_generator_new_uses_default_layout() {
    let generator = SonyflakeGenerator::new(1).expect("default machine id should be valid");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
}

#[test]
fn test_sonyflake_generator_zero_bits_select_defaults() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator =
        SonyflakeGenerator::with_clock(1, 0, 0, Duration::from_millis(10), epoch, move || epoch)
            .expect("zero bit lengths should select defaults");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
}

#[test]
fn test_sonyflake_generator_next_id_wraps_sequence_to_next_time_unit() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator =
        SonyflakeGenerator::with_clock(1, 1, 1, Duration::from_millis(1), epoch, move || {
            epoch + Duration::from_millis(5)
        })
        .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");
    let third = generator.next_id().expect("third id should generate");

    assert_eq!(generator.extract_elapsed_time(first), 5);
    assert_eq!(generator.extract_sequence(first), 0);
    assert_eq!(generator.extract_elapsed_time(second), 5);
    assert_eq!(generator.extract_sequence(second), 1);
    assert_eq!(generator.extract_elapsed_time(third), 6);
    assert_eq!(generator.extract_sequence(third), 0);
}

#[test]
fn test_sonyflake_generator_rejects_invalid_settings_and_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);

    match SonyflakeGenerator::with_epoch(65_536, epoch) {
        Err(error) => assert_eq!(
            error,
            IdError::MachineIdOutOfRange {
                machine_id: 65_536,
                max: 65_535,
            }
        ),
        Ok(_) => panic!("invalid machine id should be rejected"),
    }
    assert!(matches!(
        SonyflakeGenerator::with_options(1, 31, 1, Duration::from_millis(10), epoch),
        Err(IdError::InvalidBitLength {
            name: "sequence",
            ..
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::with_options(1, 30, 2, Duration::from_millis(10), epoch),
        Err(IdError::InvalidBitLength {
            name: "time",
            bits: 31,
            ..
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::with_clock(
            1,
            8,
            16,
            Duration::from_millis(10),
            epoch + Duration::from_millis(1),
            move || epoch,
        ),
        Err(IdError::StartTimeAhead)
    ));
    match SonyflakeGenerator::with_options(1, 8, 16, Duration::from_nanos(1), epoch) {
        Err(error) => assert_eq!(
            error,
            IdError::InvalidTimeUnit {
                nanos: 1,
                min_nanos: 1_000_000,
            }
        ),
        Ok(_) => panic!("invalid time unit should be rejected"),
    }

    let generator = SonyflakeGenerator::with_epoch(1, epoch).expect("machine id should be valid");
    assert_eq!(
        generator.compose(generator.max_elapsed_time() + 1, 0, 1),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_elapsed_time() + 1,
            max: generator.max_elapsed_time(),
        })
    );
    assert_eq!(
        generator.compose(0, generator.max_sequence() + 1, 1),
        Err(IdError::SequenceOverflow {
            sequence: generator.max_sequence() + 1,
            max: generator.max_sequence(),
        })
    );
    assert_eq!(
        generator.compose(0, 0, generator.max_machine_id() + 1),
        Err(IdError::MachineIdOutOfRange {
            machine_id: generator.max_machine_id() + 1,
            max: generator.max_machine_id(),
        })
    );
}

#[test]
fn test_sonyflake_generator_string_output_is_numeric() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator =
        SonyflakeGenerator::with_clock(7, 8, 16, Duration::from_millis(10), epoch, move || {
            epoch + Duration::from_millis(10)
        })
        .expect("configuration should be valid");

    let id = generator.next_id().expect("id should generate");

    assert_eq!(
        generator
            .next_string()
            .expect("string id should generate after numeric id"),
        (id + (1_u64 << 16)).to_string()
    );
}

#[test]
fn test_sonyflake_generator_reports_time_before_epoch_after_construction() {
    let offset = Arc::new(AtomicI64::new(0));
    let clock_offset = Arc::clone(&offset);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator =
        SonyflakeGenerator::with_clock(7, 8, 16, Duration::from_millis(10), epoch, move || {
            let millis = clock_offset.load(Ordering::SeqCst);
            if millis >= 0 {
                epoch + Duration::from_millis(millis as u64)
            } else {
                epoch - Duration::from_millis(millis.unsigned_abs())
            }
        })
        .expect("construction clock should be at epoch");

    offset.store(-1, Ordering::SeqCst);

    assert_eq!(generator.next_id(), Err(IdError::TimeBeforeEpoch));
}

#[test]
fn test_sonyflake_generator_reports_timestamp_overflow_from_clock() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator =
        SonyflakeGenerator::with_clock(7, 8, 16, Duration::from_millis(10), epoch, move || {
            epoch + Duration::from_millis((1_u64 << 39) * 10)
        })
        .expect("configuration should be valid");

    assert_eq!(
        generator.next_id(),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_elapsed_time() + 1,
            max: generator.max_elapsed_time(),
        })
    );
}
