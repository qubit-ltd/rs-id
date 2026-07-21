// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fs;
use std::path::PathBuf;

/// Verifies that the Qubit throughput benchmark measures every string-adapter
/// dispatch path alongside the numeric generator paths.
#[test]
fn test_qubit_snowflake_throughput_benchmarks_string_adapters() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let benchmark_path =
        manifest_dir.join("benches/qubit_snowflake_throughput/main.rs");
    let source = fs::read_to_string(&benchmark_path).unwrap_or_else(|error| {
        panic!(
            "failed to read benchmark source {}: {error}",
            benchmark_path.display()
        )
    });

    for fragment in [
        "SnowflakeStringGenerator",
        "sync_string_concrete",
        "sync_string_arc_dyn",
        "async_string_concrete",
        "async_string_arc_dyn",
    ] {
        assert!(
            source.contains(fragment),
            "Qubit throughput benchmark must contain `{fragment}`"
        );
    }
}
