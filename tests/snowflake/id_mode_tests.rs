/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use qubit_id::IdMode;

/// Test ID mode ordinal encoding and decoding.
#[test]
fn test_id_mode_ordinals_and_decoding() {
    assert_eq!(IdMode::Sequential.ordinal(), 0);
    assert_eq!(IdMode::Spread.ordinal(), 1);
    assert_eq!(IdMode::from_bit(0), IdMode::Sequential);
    assert_eq!(IdMode::from_bit(1), IdMode::Spread);
    assert_eq!(IdMode::from_bit(7), IdMode::Spread);
}
