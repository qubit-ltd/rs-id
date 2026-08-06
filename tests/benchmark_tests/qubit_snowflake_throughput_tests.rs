// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fs;
use std::path::PathBuf;

/// Verifies that the Qubit throughput benchmark measures ID-to-string
/// conversion paths alongside the numeric generator paths.
#[test]
fn test_qubit_snowflake_throughput_benchmarks_id_string_paths() {
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

/// Verifies that startup measurements do not include restart-fence waits.
#[test]
fn test_qubit_snowflake_throughput_startup_uses_immediate_restart_policy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let benchmark_path =
        manifest_dir.join("benches/qubit_snowflake_throughput/main.rs");
    let source = fs::read_to_string(&benchmark_path).unwrap_or_else(|error| {
        panic!(
            "failed to read benchmark source {}: {error}",
            benchmark_path.display()
        )
    });

    assert!(
        source.contains("RestartPolicy::Immediate"),
        "startup benchmark must opt out of restart-fence waits"
    );
    assert!(
        source.contains("fn measure_startup_latency"),
        "startup benchmark function must remain present"
    );
}
