// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzzes configurable Sonyflake layout compose/decode round trips.

#![no_main]

use std::time::Duration;

use libfuzzer_sys::fuzz_target;
use qubit_id::SonyflakeLayout;

/// Caps borrowed fuzzer input while retaining enough bytes for every field.
const MAX_INPUT_LEN: usize = 32;
/// Largest legal sequence or machine field width before the sum check.
const MAX_CONFIGURED_FIELD_BITS: u8 = 30;

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
    let bits_sequence = data.first().copied().unwrap_or_default() % MAX_CONFIGURED_FIELD_BITS + 1;
    let bits_machine = data.get(1).copied().unwrap_or_default() % MAX_CONFIGURED_FIELD_BITS + 1;
    if bits_sequence + bits_machine > 31 {
        return;
    }
    let machine_mask = (1_u64 << bits_machine) - 1;
    let machine_id = read_u64(data, 2) & machine_mask;
    let time_unit = Duration::from_millis(u64::from(data.get(10).copied().unwrap_or_default()) + 1);
    let layout = match SonyflakeLayout::new(machine_id, bits_sequence, bits_machine, time_unit) {
        Ok(layout) => layout,
        Err(error) => {
            panic!("bounded Sonyflake configuration must be valid: {error}")
        }
    };
    let elapsed_time = read_u64(data, 11) & layout.max_elapsed_time();
    let sequence = read_u64(data, 19) & layout.max_sequence();
    let id = match layout.compose(elapsed_time, sequence) {
        Ok(id) => id,
        Err(error) => panic!("masked Sonyflake parts must compose: {error}"),
    };
    let parts = layout.decode(id);

    assert_eq!(parts.elapsed_time(), elapsed_time);
    assert_eq!(parts.sequence(), sequence);
    assert_eq!(parts.machine_id(), machine_id);
});
