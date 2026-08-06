// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fixed-workload comparison of qubit-id UUID values and direct uuid calls.

use std::hint::black_box;
use std::time::Instant;

use qubit_id::UuidV4Generator;
use uuid::Uuid;

/// Number of untimed operations performed before each case.
const WARM_UP_ITERATIONS: usize = 25_000;
/// Number of timed samples collected for each case.
const SAMPLE_COUNT: usize = 9;
/// Number of operations performed in each timed sample.
const ITERATIONS_PER_SAMPLE: usize = 100_000;

/// Runs numeric and canonical-string UUID v4 comparison cases.
///
/// # Panics
///
/// Panics when the operating-system random source is unavailable, an ID
/// wrapper unexpectedly returns an error, or a sample has zero duration.
fn main() {
    println!(
        "configuration warm_up_iterations={WARM_UP_ITERATIONS} \
         samples={SAMPLE_COUNT} iterations_per_sample={ITERATIONS_PER_SAMPLE}"
    );

    let numeric = UuidV4Generator::new();
    run_case("qubit_id_uuid_v4_u128", || {
        numeric
            .generate()
            .expect("UUID v4 numeric generation must succeed")
            .value()
    });
    run_case("uuid_crate_v4_u128", || Uuid::new_v4().as_u128());

    run_case("qubit_id_uuid_v4_display", || {
        numeric
            .generate()
            .expect("UUID v4 string generation must succeed")
            .to_string()
    });
    run_case("uuid_crate_v4_string", || {
        Uuid::new_v4().hyphenated().to_string()
    });
}

/// Warms, measures, summarizes, and prints one benchmark case.
///
/// `T` is the generated value and `F` is the operation measured once per
/// iteration.
///
/// # Parameters
///
/// * `name` - Stable case name included in benchmark output.
/// * `operation` - Value- or string-generation operation to measure.
///
/// # Panics
///
/// Panics when a timed sample records no measurable elapsed time.
fn run_case<T, F>(name: &str, mut operation: F)
where
    F: FnMut() -> T,
{
    warm_up(&mut operation);

    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        for _ in 0..ITERATIONS_PER_SAMPLE {
            black_box(operation());
        }
        let elapsed = started.elapsed();
        assert!(
            !elapsed.is_zero(),
            "benchmark sample for {name} must have measurable duration"
        );
        samples.push(ITERATIONS_PER_SAMPLE as f64 / elapsed.as_secs_f64());
    }
    samples.sort_by(f64::total_cmp);

    println!(
        "case={name} samples={SAMPLE_COUNT} operations_per_sample={} \
         throughput_min={:.0} throughput_median={:.0} \
         throughput_max={:.0} operations/s",
        ITERATIONS_PER_SAMPLE,
        samples[0],
        samples[SAMPLE_COUNT / 2],
        samples[SAMPLE_COUNT - 1],
    );
}

/// Executes one operation repeatedly before timing begins.
///
/// # Parameters
///
/// * `operation` - Operation invoked for every warm-up iteration.
fn warm_up<T, F>(operation: &mut F)
where
    F: FnMut() -> T,
{
    for _ in 0..WARM_UP_ITERATIONS {
        black_box(operation());
    }
}
