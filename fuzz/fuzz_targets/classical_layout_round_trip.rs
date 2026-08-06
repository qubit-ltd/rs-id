// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzzes classic Snowflake layout compose/decode round trips.

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_id::ClassicalSnowflakeLayout;

/// Caps borrowed fuzzer input while retaining enough bytes for every field.
const MAX_INPUT_LEN: usize = 8;
/// Mask for the 63 bits represented by a classic Snowflake layout.
const RAW_ID_MASK: u64 = (1_u64 << 63) - 1;

/// Reads up to eight bytes, treating omitted trailing bytes as zero.
fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0_u8; 8];
    if let Some(source) = data.get(offset..) {
        let count = source.len().min(bytes.len());
        bytes[..count].copy_from_slice(&source[..count]);
    }
    u64::from_le_bytes(bytes)
}

fuzz_target!(|input: &[u8]| {
    let data = &input[..input.len().min(MAX_INPUT_LEN)];
    let raw = read_u64(data, 0) & RAW_ID_MASK;
    let parts = ClassicalSnowflakeLayout::decode_raw(raw);
    let layout = match ClassicalSnowflakeLayout::new(parts.node_id()) {
        Ok(layout) => layout,
        Err(error) => panic!(
            "decoded node identifier must create a classic layout: {error}"
        ),
    };
    let recomposed =
        match layout.compose_raw(parts.timestamp(), parts.sequence()) {
            Ok(raw) => raw,
            Err(error) => {
                panic!("decoded classic Snowflake parts must compose: {error}")
            }
        };

    assert_eq!(recomposed, raw);
});
