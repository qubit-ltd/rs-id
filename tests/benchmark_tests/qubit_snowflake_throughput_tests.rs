// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::BTreeSet;
use std::process::Command;
use std::time::Duration;

use crate::markdown_tests::process::run_with_timeout;

const EXPECTED_CASES: [&str; 12] = [
    "sync_concrete",
    "sync_arc_dyn",
    "sync_string_concrete",
    "sync_string_arc_dyn",
    "async_bridge_per_call_concrete_numeric",
    "async_bridge_per_call_dyn_numeric",
    "async_bridge_per_call_concrete_string",
    "async_bridge_per_call_dyn_string",
    "async_task_concrete_numeric",
    "async_task_dyn_numeric",
    "async_task_concrete_string",
    "async_task_dyn_string",
];

fn assert_dispatch_output(output: &str) -> Result<(), String> {
    let mut cases = BTreeSet::new();
    let mut marker_found = false;
    for line in output.lines() {
        if line == "dispatch-check passed" {
            marker_found = true;
        }
        if !line.starts_with("dispatch case=") {
            continue;
        }
        let mut case = None;
        let mut completed = None;
        for field in line.split_ascii_whitespace().skip(1) {
            if let Some(value) = field.strip_prefix("case=") {
                if case.replace(value).is_some() {
                    return Err(format!("duplicate case field in `{line}`"));
                }
            } else if let Some(value) = field.strip_prefix("completed=")
                && completed.replace(value).is_some()
            {
                return Err(format!("duplicate completed field in `{line}`"));
            }
        }
        let case = case.ok_or_else(|| format!("missing case field in `{line}`"))?;
        let completed = completed.ok_or_else(|| format!("missing completed field in `{line}`"))?;
        let completed = completed
            .parse::<usize>()
            .map_err(|_| format!("invalid completed count in `{line}`"))?;
        if completed != 128 {
            return Err(format!("case `{case}` completed {completed}, expected 128"));
        }
        if !cases.insert(case) {
            return Err(format!("duplicate dispatch case `{case}`"));
        }
    }
    let expected = EXPECTED_CASES.into_iter().collect::<BTreeSet<_>>();
    if cases != expected {
        return Err(format!("dispatch cases differ: expected {expected:?}, got {cases:?}"));
    }
    if !marker_found {
        return Err("missing `dispatch-check passed` marker".to_owned());
    }
    Ok(())
}

fn valid_dispatch_output() -> String {
    let results = EXPECTED_CASES
        .iter()
        .map(|case| format!("dispatch case={case} completed=128 elapsed_s=0.1 throughput=1280 operations/s"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{results}\ndispatch-check passed\n")
}

#[test]
fn test_dispatch_output_parser_accepts_exact_cases_and_ignores_cargo_logs() {
    let output = format!(
        "   Compiling qubit-id v0.1.0\n{}Finished bench profile\n",
        valid_dispatch_output()
    );
    assert!(assert_dispatch_output(&output).is_ok());
}

#[test]
fn test_dispatch_output_parser_rejects_invalid_results() {
    let valid = valid_dispatch_output();
    let lines = valid.lines().map(str::to_owned).collect::<Vec<_>>();
    let missing = lines[..11].join("\n") + "\ndispatch-check passed";
    let duplicate = format!("{}\n{}", valid, lines[0]);
    let wrong_count = valid.replace("completed=128", "completed=127");
    let non_integer_count = valid.replace("completed=128", "completed=many");
    let missing_field = valid.replace(" completed=128", "");
    let duplicate_field = valid.replacen("completed=128", "completed=128 completed=128", 1);
    let extra = format!(
        "{}\ndispatch case=unknown completed=128 elapsed_s=0.1 throughput=1280 operations/s",
        valid
    );
    let no_marker = lines[..12].join("\n");

    for invalid in [
        missing,
        duplicate,
        wrong_count,
        non_integer_count,
        missing_field,
        duplicate_field,
        extra,
        no_marker,
    ] {
        assert!(
            assert_dispatch_output(&invalid).is_err(),
            "accepted invalid output:\n{invalid}"
        );
    }
}

#[test]
#[ignore = "run once by the project hook"]
fn test_review_benchmark_dispatch() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_nanos();
    let log_dir = std::env::temp_dir().join(format!("qubit-dispatch-{}-{run_id}", std::process::id()));
    std::fs::create_dir_all(&log_dir).expect("benchmark log directory must be created");
    let log_path = log_dir.join("dispatch.log");
    let result = run_review_benchmark(manifest_dir, &log_path);
    match result {
        Ok(()) => std::fs::remove_dir_all(&log_dir).expect("successful benchmark logs must be cleaned up"),
        Err(error) => panic!("{error}; log retained at {}", log_path.display()),
    }
}

fn run_review_benchmark(manifest_dir: &str, log_path: &std::path::Path) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.current_dir(manifest_dir).args([
        "bench",
        "--locked",
        "--bench",
        "qubit_snowflake_throughput",
        "--no-default-features",
        "--features",
        "qubit-snowflake",
        "--",
        "--check-dispatch",
    ]);
    let outcome = run_with_timeout(command, Duration::from_secs(600), "benchmark dispatch", log_path)?;
    if !outcome.status.success() {
        return Err(format!(
            "benchmark dispatch exited with {}; log: {}",
            outcome.status,
            outcome.log_path.display()
        ));
    }
    let output = std::fs::read_to_string(log_path)
        .map_err(|error| format!("failed to read benchmark log {}: {error}", log_path.display()))?;
    assert_dispatch_output(&output).map_err(|error| format!("{error}; log: {}", log_path.display()))
}
