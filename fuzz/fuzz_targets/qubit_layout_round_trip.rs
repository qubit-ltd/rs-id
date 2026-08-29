// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzzes Qubit Snowflake layout compose/decode round trips.

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_id::SnowflakeLayout;

/// Caps borrowed fuzzer input to one raw Qubit Snowflake bit pattern.
const MAX_INPUT_LEN: usize = 8;

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
    let raw = read_u64(data, 0);
    let parts = SnowflakeLayout::decode_raw(raw);
    let layout = match SnowflakeLayout::new(parts.mode(), parts.precision(), parts.host()) {
        Ok(layout) => layout,
        Err(error) => {
            panic!("decoded Qubit fields must create a layout: {error}")
        }
    };
    let recomposed = match layout.compose_raw(parts.timestamp(), parts.sequence()) {
        Ok(raw) => raw,
        Err(error) => panic!("decoded Qubit parts must compose: {error}"),
    };

    assert_eq!(recomposed, raw);
});
