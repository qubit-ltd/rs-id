/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for `IdError` formatting.

use std::error::Error;

use qubit_id::IdError;

fn assert_error_trait(error: &dyn Error) -> String {
    error.to_string()
}

#[test]
fn test_id_error_display_formats_all_variants() {
    let cases = [
        (
            IdError::HostOutOfRange { host: 512, max: 511 },
            "host id 512 is out of range 0..=511",
        ),
        (
            IdError::NodeOutOfRange {
                node_id: 1024,
                max: 1023,
            },
            "node id 1024 is out of range 0..=1023",
        ),
        (
            IdError::MachineIdOutOfRange {
                machine_id: 65_536,
                max: 65_535,
            },
            "machine id 65536 is out of range 0..=65535",
        ),
        (
            IdError::TimestampOverflow { timestamp: 8, max: 7 },
            "timestamp 8 exceeds maximum 7",
        ),
        (
            IdError::SequenceOverflow { sequence: 4, max: 3 },
            "sequence 4 exceeds maximum 3",
        ),
        (
            IdError::ClockMovedBackwards {
                last_timestamp: 10,
                current_timestamp: 9,
                skew_millis: 1,
                max_skew_millis: 0,
            },
            "clock moved backwards from 10 to 9; skew 1 ms exceeds maximum 0 ms",
        ),
        (IdError::TimeBeforeEpoch, "time is before the configured epoch"),
        (IdError::StartTimeAhead, "start time is ahead of the generator clock"),
        (
            IdError::InvalidBitLength {
                name: "time",
                bits: 31,
                reason: "time bit length must be at least 32",
            },
            "invalid bit length for time: 31; time bit length must be at least 32",
        ),
        (
            IdError::InvalidTimeUnit {
                nanos: 1,
                min_nanos: 1_000_000,
            },
            "invalid time unit 1 ns; minimum is 1000000 ns",
        ),
        (
            IdError::RandomSourceUnavailable,
            "operating system random source is unavailable",
        ),
        (IdError::StatePoisoned, "generator state mutex is poisoned"),
    ];

    for (error, expected) in cases {
        assert_eq!(assert_error_trait(&error), expected);
    }
}
