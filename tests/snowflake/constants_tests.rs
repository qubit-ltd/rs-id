/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_id::{
    DEFAULT_MAX_SKEW_MILLIS,
    HOST_BITS,
    HOST_MAX,
    HOST_MIN,
    PRECISION_BITS,
};

/// Test public Qubit snowflake layout constants.
#[test]
fn test_snowflake_public_constants_match_layout() {
    assert_eq!(HOST_BITS, 9);
    assert_eq!(HOST_MIN, 0);
    assert_eq!(HOST_MAX, 511);
    assert_eq!(PRECISION_BITS, 1);
    assert_eq!(DEFAULT_MAX_SKEW_MILLIS, 3_000);
}
