// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for `IdError` formatting and error sources.

use std::error::Error;
use std::time::{
    Duration,
    UNIX_EPOCH,
};

use qubit_id::IdError;

/// Formats an error through the standard error trait.
///
/// # Parameters
///
/// * `error` - Error trait object to format.
///
/// # Returns
///
/// The error's display representation.
#[inline(always)]
fn assert_error_trait(error: &dyn Error) -> String {
    error.to_string()
}

#[test]
fn test_id_error_display_formats_all_variants() {
    let epoch = UNIX_EPOCH + Duration::from_secs(10);
    let time = UNIX_EPOCH + Duration::from_secs(9);
    let cases = vec![
        (
            IdError::HostOutOfRange {
                host: 512,
                max: 511,
            },
            "host id 512 is out of range 0..=511".to_owned(),
        ),
        (
            IdError::NodeOutOfRange {
                node_id: 1024,
                max: 1023,
            },
            "node id 1024 is out of range 0..=1023".to_owned(),
        ),
        (
            IdError::MachineIdOutOfRange {
                machine_id: 65_536,
                max: 65_535,
            },
            "machine id 65536 is out of range 0..=65535".to_owned(),
        ),
        (
            IdError::TimestampOverflow {
                timestamp: 8,
                max: 7,
            },
            "timestamp 8 exceeds maximum 7".to_owned(),
        ),
        (
            IdError::SequenceOverflow {
                sequence: 4,
                max: 3,
            },
            "sequence 4 exceeds maximum 3".to_owned(),
        ),
        (
            IdError::ClockMovedBackwards {
                last_elapsed: Duration::from_secs(10),
                current_elapsed: Duration::from_secs(9),
                skew: Duration::from_secs(1),
                max_skew: Duration::ZERO,
            },
            "clock moved backwards from 10s to 9s; skew 1s exceeds maximum 0ns".to_owned(),
        ),
        (
            IdError::TimeBeforeEpoch { time, epoch },
            format!("time {time:?} is before the configured epoch {epoch:?}"),
        ),
        (
            IdError::StartTimeAhead {
                start_time: epoch,
                current_time: time,
            },
            format!("start time {epoch:?} is ahead of generator clock {time:?}"),
        ),
        (
            IdError::InvalidBitLength {
                name: "time",
                bits: 31,
                reason: "time bit length must be at least 32",
            },
            "invalid bit length for time: 31; time bit length must be at least 32".to_owned(),
        ),
        (
            IdError::InvalidTimeUnit {
                nanos: 1,
                min_nanos: 1_000_000,
            },
            "invalid time unit 1 ns; minimum is 1000000 ns".to_owned(),
        ),
        (
            IdError::ExpirationTimeOverflow {
                origin: epoch,
                time_unit: Duration::from_secs(1),
                max_timestamp: 7,
            },
            format!(
                "expiration time overflows SystemTime for origin {epoch:?}, \
                time unit 1s, and maximum timestamp 7"
            ),
        ),
        (
            IdError::GeneratorExpired {
                observed_at: epoch,
                expires_at: time,
            },
            format!(
                "generator expired at {time:?}; observed wall time was {epoch:?}"
            ),
        ),
    ];

    #[cfg(any(
        feature = "qubit-snowflake",
        feature = "classic-snowflake",
        feature = "sonyflake",
    ))]
    let cases = {
        let mut cases = cases;
        cases.push((
            IdError::WaitFailed {
                source: qubit_clock::TimeError::InstantOverflow,
            },
            "failed to wait before retrying ID generation".to_owned(),
        ));
        cases
    };

    for (error, expected) in cases {
        assert_eq!(assert_error_trait(&error), expected);
    }
}

#[test]
fn test_id_error_clock_moved_backwards_preserves_raw_durations() {
    let error = IdError::ClockMovedBackwards {
        last_elapsed: Duration::from_millis(10_500),
        current_elapsed: Duration::from_millis(10_400),
        skew: Duration::from_millis(100),
        max_skew: Duration::from_secs(3),
    };

    assert!(error.to_string().contains("100ms"));
}

#[test]
fn test_id_error_preserves_sources() {
    #[cfg(any(
        feature = "qubit-snowflake",
        feature = "classic-snowflake",
        feature = "sonyflake",
    ))]
    {
        let wait = IdError::WaitFailed {
            source: qubit_clock::TimeError::InstantOverflow,
        };
        assert!(Error::source(&wait).is_some());
    }
}

#[cfg(feature = "uuid")]
#[test]
fn test_id_error_random_source_failed_preserves_source() {
    let error = IdError::RandomSourceFailed {
        source: getrandom::Error::UNSUPPORTED,
    };

    assert_eq!(
        error.to_string(),
        "failed to obtain random bytes for UUID v4"
    );
    assert!(Error::source(&error).is_some());
}
