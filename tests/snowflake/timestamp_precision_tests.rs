/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_id::TimestampPrecision;

/// Test timestamp precision encoding and layout properties.
#[test]
fn test_timestamp_precision_layout_values() {
    assert_eq!(TimestampPrecision::Millisecond.ordinal(), 0);
    assert_eq!(TimestampPrecision::Second.ordinal(), 1);
    assert_eq!(
        TimestampPrecision::from_bit(0),
        TimestampPrecision::Millisecond,
    );
    assert_eq!(TimestampPrecision::from_bit(1), TimestampPrecision::Second);
    assert_eq!(TimestampPrecision::from_bit(9), TimestampPrecision::Second);

    assert_eq!(TimestampPrecision::Millisecond.timestamp_bits(), 41);
    assert_eq!(TimestampPrecision::Millisecond.sequence_bits(), 12);
    assert_eq!(TimestampPrecision::Millisecond.divisor_millis(), 1);
    assert_eq!(TimestampPrecision::Millisecond.wait_duration_millis(), 1);

    assert_eq!(TimestampPrecision::Second.timestamp_bits(), 31);
    assert_eq!(TimestampPrecision::Second.sequence_bits(), 22);
    assert_eq!(TimestampPrecision::Second.divisor_millis(), 1_000);
    assert_eq!(TimestampPrecision::Second.wait_duration_millis(), 500);
}
