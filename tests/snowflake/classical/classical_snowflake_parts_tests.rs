// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for decoded classic Snowflake parts.

use qubit_id::ClassicalSnowflakeLayout;

#[test]
fn test_snowflake_parts_accessors_return_decoded_fields() {
    let layout = ClassicalSnowflakeLayout::new(9)
        .expect("node id must fit the classic layout");
    let id = layout
        .compose(42, 7)
        .expect("parts must fit the classic layout");
    let parts = ClassicalSnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 42);
    assert_eq!(parts.node_id(), 9);
    assert_eq!(parts.sequence(), 7);
}
