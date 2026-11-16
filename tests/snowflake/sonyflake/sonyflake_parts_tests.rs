// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for decoded Sonyflake parts.

use std::time::Duration;

use qubit_id::SonyflakeLayout;

#[test]
fn test_sonyflake_parts_accessors_return_decoded_fields() {
    let layout = SonyflakeLayout::new(9, 8, 16, Duration::from_millis(10)).expect("Sonyflake layout must be valid");
    let id = layout.compose(42, 7).expect("parts must fit the Sonyflake layout");
    let parts = layout.decode(id);

    assert_eq!(parts.elapsed_time(), 42);
    assert_eq!(parts.sequence(), 7);
    assert_eq!(parts.machine_id(), 9);
}
