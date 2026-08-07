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

/// Verifies that dynamic benchmark paths use the generic generator parameters
/// introduced by the generator trait refactor.
#[test]
fn test_qubit_snowflake_throughput_uses_generic_generator_parameters() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let benchmark_path =
        manifest_dir.join("benches/qubit_snowflake_throughput/main.rs");
    let source = fs::read_to_string(&benchmark_path).unwrap_or_else(|error| {
        panic!(
            "failed to read benchmark source {}: {error}",
            benchmark_path.display()
        )
    });

    assert_eq!(
        source
            .matches("dyn IdGenerator<Id, IdGenerationError>")
            .count(),
        2,
        "blocking benchmark paths must use generic generator parameters"
    );
    assert_eq!(
        source
            .matches("dyn AsyncIdGenerator<Id, IdGenerationError>")
            .count(),
        2,
        "async benchmark paths must use generic generator parameters"
    );
    assert!(
        !source.contains("Output = Id, Error = IdGenerationError"),
        "benchmark must not use removed associated type syntax"
    );
}

/// Verifies that the UUID comparison benchmark invokes the blocking capability
/// rather than the UUID generator's inherent convenience method.
#[test]
fn test_uuid_comparison_benchmark_uses_blocking_generator_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let benchmark_path = manifest_dir.join("benches/uuid_comparison/main.rs");
    let source = fs::read_to_string(&benchmark_path).unwrap_or_else(|error| {
        panic!(
            "failed to read benchmark source {}: {error}",
            benchmark_path.display()
        )
    });

    assert!(
        source.contains("IdGenerator::generate(&numeric)"),
        "UUID comparison benchmark must invoke IdGenerator explicitly"
    );
}
