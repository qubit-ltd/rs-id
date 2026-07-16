// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Sustained-throughput benchmark executable for the Qubit Snowflake generator.

mod startup_latency_summary;
mod throughput_sample;
mod throughput_summary;

use std::hint::black_box;
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
};

use qubit_id::{
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    TimestampPrecision,
};

use self::startup_latency_summary::StartupLatencySummary;
use self::throughput_sample::ThroughputSample;
use self::throughput_summary::ThroughputSummary;

const HOST: u64 = 0;
const WORKER_COUNTS: [usize; 4] = [1, 2, 4, 6];
const SAMPLE_COUNT: usize = 3;
const STARTUP_SAMPLE_COUNT: usize = 10_000;
const WARM_UP_IDS: usize = 100_000;
const MILLIS_SLICES: u64 = 2_000;
const SECOND_SLICES: u64 = 2;
const BATCH_SIZE: usize = 64;

/// Runs every precision and worker-count benchmark case.
///
/// # Panics
///
/// Panics when benchmark configuration, ID generation, worker execution, or
/// system-clock conversion fails.
fn main() {
    println!(
        "configuration throughput_samples={SAMPLE_COUNT} \
         startup_samples={STARTUP_SAMPLE_COUNT} warm_up_ids={WARM_UP_IDS}"
    );

    for precision in
        [TimestampPrecision::Millisecond, TimestampPrecision::Second]
    {
        for worker_count in WORKER_COUNTS {
            let summary = summarize_case(precision, worker_count);
            println!(
                "throughput precision={} threads={} samples={} slices={} \
                 capacity={} median_count={} median_utilization={:.2}% \
                 median_elapsed_s={:.6} throughput_min={:.0} \
                 throughput_median={:.0} throughput_max={:.0} ids/s",
                precision_name(precision),
                worker_count,
                SAMPLE_COUNT,
                slice_count(precision),
                summary.median.capacity,
                summary.median.generated,
                summary.median.utilization(),
                summary.median.elapsed.as_secs_f64(),
                summary.min.throughput(),
                summary.median.throughput(),
                summary.max.throughput(),
            );
        }
    }

    for precision in
        [TimestampPrecision::Millisecond, TimestampPrecision::Second]
    {
        let summary = measure_startup_latency(precision);
        println!(
            "startup precision={} samples={} latency_min_ns={} \
             latency_median_ns={} latency_max_ns={}",
            precision_name(precision),
            STARTUP_SAMPLE_COUNT,
            summary.min_nanos,
            summary.median_nanos,
            summary.max_nanos,
        );
    }
}

/// Runs and summarizes repeated samples for one benchmark case.
///
/// # Arguments
///
/// * `precision` - Timestamp precision to measure.
/// * `worker_count` - Number of concurrent generator workers.
///
/// # Returns
///
/// Minimum, median, and maximum samples ordered by throughput.
///
/// # Panics
///
/// Panics under the same conditions as [`measure_throughput`].
fn summarize_case(
    precision: TimestampPrecision,
    worker_count: usize,
) -> ThroughputSummary {
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        samples.push(measure_throughput(precision, worker_count));
    }
    samples.sort_by(|left, right| {
        left.throughput().total_cmp(&right.throughput())
    });

    ThroughputSummary {
        min: samples[0],
        median: samples[SAMPLE_COUNT / 2],
        max: samples[SAMPLE_COUNT - 1],
    }
}

/// Measures one precision and worker-count sample.
///
/// Generator construction and warm-up happen before timing. Worker threads
/// wait at a barrier while the main thread aligns the run to a fresh clock
/// slice. The returned count is checked against the theoretical capacity for
/// the measured slices.
///
/// # Arguments
///
/// * `precision` - Timestamp precision to measure.
/// * `worker_count` - Number of concurrent generator workers.
///
/// # Returns
///
/// Generated count, theoretical capacity, and elapsed wall duration.
///
/// # Panics
///
/// Panics when generator construction or generation fails, a worker panics,
/// no ID is measured, or the observed count exceeds theoretical capacity.
fn measure_throughput(
    precision: TimestampPrecision,
    worker_count: usize,
) -> ThroughputSample {
    let generator = Arc::new(
        QubitSnowflakeGenerator::builder(HOST)
            .mode(IdMode::Sequential)
            .precision(precision)
            .build()
            .expect("benchmark generator configuration must be valid"),
    );
    warm_up(&generator);
    let epoch = generator.epoch();
    let measured_slices = slice_count(precision);
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
                measured_slices,
            )
        }));
    }

    let aligned_timestamp = wait_for_fresh_slice(epoch, precision);
    start_timestamp.store(aligned_timestamp, Ordering::Release);
    let started = Instant::now();
    barrier.wait();

    let mut generated = 0_u64;
    for worker in workers {
        generated += worker.join().expect("benchmark worker must not panic");
    }
    let elapsed = started.elapsed();
    let capacity = measured_slices * (1_u64 << precision.sequence_bits());
    assert!(
        generated <= capacity,
        "generated count {generated} exceeds capacity {capacity}"
    );
    assert!(
        generated > 0,
        "benchmark sample must generate at least one ID"
    );

    ThroughputSample {
        generated,
        capacity,
        elapsed,
    }
}

/// Generates untimed IDs so one-time setup does not influence throughput.
///
/// # Arguments
///
/// * `generator` - Generator to warm before measurement.
///
/// # Panics
///
/// Panics when warm-up ID generation fails.
fn warm_up(generator: &QubitSnowflakeGenerator) {
    for _ in 0..WARM_UP_IDS {
        black_box(
            generator
                .next_id()
                .expect("warm-up ID generation must succeed"),
        );
    }
}

/// Measures construction plus the first ID generation on fresh instances.
///
/// # Arguments
///
/// * `precision` - Timestamp precision used by every fresh instance.
///
/// # Returns
///
/// Minimum, median, and maximum startup latency in nanoseconds.
///
/// # Panics
///
/// Panics when generator construction or first-ID generation fails.
fn measure_startup_latency(
    precision: TimestampPrecision,
) -> StartupLatencySummary {
    let mut samples = Vec::with_capacity(STARTUP_SAMPLE_COUNT);
    for _ in 0..STARTUP_SAMPLE_COUNT {
        let started = Instant::now();
        let generator = QubitSnowflakeGenerator::builder(HOST)
            .mode(IdMode::Sequential)
            .precision(precision)
            .build()
            .expect("benchmark generator configuration must be valid");
        black_box(
            generator
                .next_id()
                .expect("first ID generation must succeed"),
        );
        samples.push(started.elapsed().as_nanos());
    }
    samples.sort_unstable();

    StartupLatencySummary {
        min_nanos: samples[0],
        median_nanos: samples[STARTUP_SAMPLE_COUNT / 2],
        max_nanos: samples[STARTUP_SAMPLE_COUNT - 1],
    }
}

/// Generates IDs until the physical clock reaches the target slice.
///
/// IDs are produced in fixed-size batches. Only the final boundary batch is
/// decoded, keeping timestamp extraction outside the hot path for earlier
/// batches while excluding IDs outside the measured timestamp range.
///
/// # Arguments
///
/// * `generator` - Shared generator used by this worker.
/// * `epoch` - Timestamp origin configured on `generator`.
/// * `precision` - Timestamp precision configured on `generator`.
/// * `start_timestamp` - First logical slice included in the sample.
/// * `measured_slices` - Number of logical slices included in the sample.
///
/// # Returns
///
/// Number of IDs generated inside the measured timestamp range.
///
/// # Panics
///
/// Panics when ID generation or system-clock conversion fails.
fn generate_until_target(
    generator: &QubitSnowflakeGenerator,
    epoch: SystemTime,
    precision: TimestampPrecision,
    start_timestamp: u64,
    measured_slices: u64,
) -> u64 {
    let target_timestamp = start_timestamp + measured_slices;
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
/// This function can wait indefinitely if the system wall clock does not
/// enter a later logical slice.
///
/// # Arguments
///
/// * `epoch` - Timestamp origin used for clock conversion.
/// * `precision` - Logical timestamp precision to observe.
///
/// # Returns
///
/// The first observed timestamp after the initial logical slice.
///
/// # Panics
///
/// Panics when the epoch is ahead of the system clock or the timestamp does
/// not fit in `u64`.
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
///
/// # Arguments
///
/// * `epoch` - Timestamp origin used for clock conversion.
/// * `precision` - Logical timestamp precision to apply.
///
/// # Returns
///
/// Current logical timestamp elapsed since `epoch`.
///
/// # Panics
///
/// Panics when `epoch` is ahead of the system clock or the timestamp does not
/// fit in `u64`.
fn current_timestamp(epoch: SystemTime, precision: TimestampPrecision) -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(epoch)
        .expect("benchmark epoch must be in the past");
    let timestamp =
        elapsed.as_millis() / u128::from(precision.divisor_millis());
    u64::try_from(timestamp).expect("benchmark timestamp must fit in u64")
}

/// Returns the number of complete slices measured for a precision.
///
/// # Arguments
///
/// * `precision` - Timestamp precision whose benchmark window is requested.
///
/// # Returns
///
/// Number of complete logical slices in one throughput sample.
#[inline(always)]
fn slice_count(precision: TimestampPrecision) -> u64 {
    match precision {
        TimestampPrecision::Millisecond => MILLIS_SLICES,
        TimestampPrecision::Second => SECOND_SLICES,
    }
}

/// Returns the stable display name for a timestamp precision.
///
/// # Arguments
///
/// * `precision` - Timestamp precision to name.
///
/// # Returns
///
/// Stable lowercase name used in benchmark output.
#[inline(always)]
fn precision_name(precision: TimestampPrecision) -> &'static str {
    match precision {
        TimestampPrecision::Millisecond => "millisecond",
        TimestampPrecision::Second => "second",
    }
}
