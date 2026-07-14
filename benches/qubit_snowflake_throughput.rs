// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Sustained-throughput benchmark for the Qubit Snowflake generator.

use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::sync::{
    Arc,
    Barrier,
};
use std::thread;
use std::time::{
    Duration,
    Instant,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_id::{
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    TimestampPrecision,
};

const HOST: u64 = 0;
const WORKER_COUNTS: [usize; 4] = [1, 2, 4, 6];
const MILLIS_SLICES: u64 = 5_000;
const SECOND_SLICES: u64 = 5;
const BATCH_SIZE: usize = 64;

/// Runs every precision and worker-count benchmark case.
fn main() {
    for precision in
        [TimestampPrecision::Millisecond, TimestampPrecision::Second]
    {
        for worker_count in WORKER_COUNTS {
            run_case(precision, worker_count);
        }
    }
}

/// Measures one precision and worker-count combination.
///
/// Generator construction and the startup fence happen before timing. Worker
/// threads wait at a barrier while the main thread aligns the run to a fresh
/// clock slice. The function prints the generated count, theoretical capacity,
/// utilization, elapsed time, and throughput after all workers finish.
fn run_case(precision: TimestampPrecision, worker_count: usize) {
    let epoch = UNIX_EPOCH;
    let generator = Arc::new(
        QubitSnowflakeGenerator::with_options(
            IdMode::Sequential,
            precision,
            HOST,
            epoch,
        )
        .expect("benchmark generator configuration must be valid"),
    );
    generator
        .next_id()
        .expect("benchmark startup fence must complete");

    let slice_count = slice_count(precision);
    let barrier = Arc::new(Barrier::new(worker_count + 1));
    let start_timestamp = Arc::new(AtomicU64::new(0));
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let generator = Arc::clone(&generator);
        let barrier = Arc::clone(&barrier);
        let start_timestamp = Arc::clone(&start_timestamp);
        workers.push(thread::spawn(move || {
            barrier.wait();
            let start_timestamp = start_timestamp.load(Ordering::Acquire);
            generate_until_target(
                &generator,
                epoch,
                precision,
                start_timestamp,
                slice_count,
            )
        }));
    }

    let aligned_timestamp = wait_for_fresh_slice(epoch, precision);
    let target_timestamp = aligned_timestamp + slice_count;
    start_timestamp.store(aligned_timestamp, Ordering::Release);
    let started = Instant::now();
    barrier.wait();

    let mut generated = 0_u64;
    for worker in workers {
        generated += worker.join().expect("benchmark worker must not panic");
    }
    let elapsed = started.elapsed();
    let capacity = slice_count * (1_u64 << precision.sequence_bits());
    let utilization = generated as f64 * 100.0 / capacity as f64;
    let throughput = generated as f64 / elapsed.as_secs_f64();

    println!(
        "precision={} threads={} slices={} start_timestamp={} \
         target_timestamp={} count={} capacity={} utilization={:.2}% \
         elapsed_s={:.6} throughput={:.0} ids/s",
        precision_name(precision),
        worker_count,
        slice_count,
        aligned_timestamp,
        target_timestamp,
        generated,
        capacity,
        utilization,
        elapsed.as_secs_f64(),
        throughput,
    );
}

/// Generates IDs until the physical clock reaches the target slice.
///
/// IDs are produced in fixed-size batches. Only the final boundary batch is
/// decoded, keeping timestamp extraction outside the hot path for earlier
/// batches while excluding IDs outside the measured timestamp range.
fn generate_until_target(
    generator: &QubitSnowflakeGenerator,
    epoch: SystemTime,
    precision: TimestampPrecision,
    start_timestamp: u64,
    slice_count: u64,
) -> u64 {
    let target_timestamp = start_timestamp + slice_count;
    let mut generated = 0_u64;
    let mut batch = [0_u64; BATCH_SIZE];

    loop {
        for id in &mut batch {
            *id = generator
                .next_id()
                .expect("ID generation must succeed during the benchmark");
        }
        if current_timestamp(epoch, precision) < target_timestamp {
            generated += BATCH_SIZE as u64;
            continue;
        }
        generated += batch
            .iter()
            .filter(|id| {
                let timestamp = QubitSnowflakeLayout::decode(**id).timestamp();
                (start_timestamp..target_timestamp).contains(&timestamp)
            })
            .count() as u64;
        return generated;
    }
}

/// Waits until the system clock enters a new precision-specific slice.
///
/// The short sleeps avoid consuming a CPU core while retaining sufficiently
/// precise alignment for both supported precisions.
fn wait_for_fresh_slice(
    epoch: SystemTime,
    precision: TimestampPrecision,
) -> u64 {
    let initial = current_timestamp(epoch, precision);
    let sleep_duration = match precision {
        TimestampPrecision::Millisecond => Duration::from_micros(50),
        TimestampPrecision::Second => Duration::from_millis(1),
    };
    loop {
        thread::sleep(sleep_duration);
        let current = current_timestamp(epoch, precision);
        if current > initial {
            return current;
        }
    }
}

/// Returns the current elapsed timestamp for the selected precision.
fn current_timestamp(epoch: SystemTime, precision: TimestampPrecision) -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(epoch)
        .expect("benchmark epoch must be in the past");
    let timestamp =
        elapsed.as_millis() / u128::from(precision.divisor_millis());
    u64::try_from(timestamp).expect("benchmark timestamp must fit in u64")
}

/// Returns the number of complete slices measured for a precision.
fn slice_count(precision: TimestampPrecision) -> u64 {
    match precision {
        TimestampPrecision::Millisecond => MILLIS_SLICES,
        TimestampPrecision::Second => SECOND_SLICES,
    }
}

/// Returns the stable display name for a timestamp precision.
fn precision_name(precision: TimestampPrecision) -> &'static str {
    match precision {
        TimestampPrecision::Millisecond => "millisecond",
        TimestampPrecision::Second => "second",
    }
}
