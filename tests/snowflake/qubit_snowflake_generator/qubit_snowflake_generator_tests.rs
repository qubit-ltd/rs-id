// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Qubit snowflake generator.

use std::collections::HashSet;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::atomic::{
    AtomicBool,
    AtomicU64,
    Ordering,
};
use std::sync::{
    Arc,
    Condvar,
    Mutex,
};
use std::thread;
use std::time::{
    Duration,
    Instant,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_id::{
    DEFAULT_MAX_SKEW_MILLIS,
    IdError,
    IdGenerator,
    IdMode,
    QubitSnowflakeGenerator,
    QubitSnowflakeLayout,
    TimestampPrecision,
};

/// Clock that keeps two overflow waiters in one time slice until released.
struct CoordinatedClock {
    epoch: SystemTime,
    call_count: AtomicU64,
    current_millis: AtomicU64,
    race_started: AtomicBool,
    workers: Mutex<HashSet<thread::ThreadId>>,
    workers_changed: Condvar,
}

impl CoordinatedClock {
    /// Creates a clock whose first reading is timestamp 9 and later readings
    /// remain at timestamp 10 until explicitly advanced.
    fn new(epoch: SystemTime) -> Self {
        Self {
            epoch,
            call_count: AtomicU64::new(0),
            current_millis: AtomicU64::new(10),
            race_started: AtomicBool::new(false),
            workers: Mutex::new(HashSet::new()),
            workers_changed: Condvar::new(),
        }
    }

    /// Returns the current test time and records overflow worker threads.
    fn now(&self) -> SystemTime {
        let call = self.call_count.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            return self.epoch + Duration::from_millis(9);
        }
        if self.race_started.load(Ordering::SeqCst) {
            let mut workers = self
                .workers
                .lock()
                .expect("test clock worker set should not be poisoned");
            if workers.insert(thread::current().id()) {
                self.workers_changed.notify_all();
            }
        }
        self.epoch
            + Duration::from_millis(self.current_millis.load(Ordering::SeqCst))
    }

    /// Starts recording threads that read the clock during sequence overflow.
    fn start_race(&self) {
        self.race_started.store(true, Ordering::SeqCst);
    }

    /// Waits until the expected number of overflow workers have read the clock.
    fn wait_for_workers(&self, expected: usize) {
        let workers = self
            .workers
            .lock()
            .expect("test clock worker set should not be poisoned");
        let (workers, _) = self
            .workers_changed
            .wait_timeout_while(workers, Duration::from_secs(2), |workers| {
                workers.len() < expected
            })
            .expect("test clock wait should not be poisoned");
        assert_eq!(
            workers.len(),
            expected,
            "all overflow workers should reach the clock"
        );
    }

    /// Advances the clock to the specified millisecond timestamp.
    fn advance_to(&self, timestamp: u64) {
        self.current_millis.store(timestamp, Ordering::SeqCst);
    }
}

#[test]
fn test_generate_at_matches_layout_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        7,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch + Duration::from_millis(123),
    )
    .expect("configuration should be valid");

    let id = generator
        .generate_at(epoch + Duration::from_millis(45), 9)
        .expect("timestamp and sequence should be valid");

    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.timestamp(), 45);
    assert_eq!(parts.sequence(), 9);
    assert_eq!(parts.host(), 7);
    assert_eq!(generator.epoch(), epoch);
}

#[test]
fn test_qubit_snowflake_generator_accessors_return_configuration() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        17,
        epoch,
        37,
        move || epoch + Duration::from_millis(100),
    )
    .expect("configuration should be valid");
    let expected_layout = QubitSnowflakeLayout::new(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        17,
    )
    .expect("layout should be valid");

    assert_eq!(generator.layout(), &expected_layout);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.max_skew_millis(), 37);
}

#[test]
fn test_qubit_snowflake_generator_next_id_increments_sequence_in_same_slice() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || {
            let timestamp = if clock_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                10
            } else {
                11
            };
            epoch + Duration::from_millis(timestamp)
        },
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");

    assert_eq!(QubitSnowflakeLayout::decode(first).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(second).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
    assert_eq!(
        generator.next_string().expect("string id should generate"),
        second.wrapping_add(1).to_string()
    );
}

#[test]
fn test_qubit_snowflake_generator_reports_large_clock_backwards() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let current_millis = Arc::new(AtomicU64::new(10));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        0,
        move || {
            let timestamp = if clock_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                9
            } else {
                clock_millis.load(Ordering::SeqCst)
            };
            epoch + Duration::from_millis(timestamp)
        },
    )
    .expect("configuration should be valid");

    generator.next_id().expect("first id should generate");
    current_millis.store(9, Ordering::SeqCst);

    assert_eq!(
        generator.next_id(),
        Err(IdError::ClockMovedBackwards {
            last_timestamp: 10,
            current_timestamp: 9,
            skew_millis: 1,
            max_skew_millis: 0,
        })
    );
}

#[test]
fn test_qubit_snowflake_generator_waits_for_small_clock_backwards() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        2,
        move || {
            let call = clock_calls.fetch_add(1, Ordering::SeqCst);
            match call {
                0 => epoch + Duration::from_millis(9),
                1 | 2 => epoch + Duration::from_millis(10),
                3 => epoch + Duration::from_millis(9),
                _ => epoch + Duration::from_millis(10),
            }
        },
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator
        .next_id()
        .expect("small clock skew should wait and retry");

    assert_eq!(QubitSnowflakeLayout::decode(first).sequence(), 0);
    assert_eq!(QubitSnowflakeLayout::decode(second).sequence(), 1);
}

#[test]
fn test_qubit_snowflake_generator_waits_when_sequence_overflows() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || {
            let call = clock_calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                epoch + Duration::from_millis(9)
            } else if call <= 4_097 {
                epoch + Duration::from_millis(10)
            } else {
                epoch + Duration::from_millis(11)
            }
        },
    )
    .expect("configuration should be valid");

    for expected_sequence in 0..=4_095 {
        let id = generator.next_id().expect("id should generate");
        assert_eq!(
            QubitSnowflakeLayout::decode(id).sequence(),
            expected_sequence
        );
    }
    let wrapped = generator
        .next_id()
        .expect("generator should wait for the next timestamp");

    assert_eq!(QubitSnowflakeLayout::decode(wrapped).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(wrapped).sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_skips_initial_time_slice() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let current_millis = Arc::new(AtomicU64::new(10));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = Arc::new(
        QubitSnowflakeGenerator::with_clock(
            IdMode::Sequential,
            TimestampPrecision::Millisecond,
            3,
            epoch,
            DEFAULT_MAX_SKEW_MILLIS,
            move || {
                clock_calls.fetch_add(1, Ordering::SeqCst);
                epoch
                    + Duration::from_millis(clock_millis.load(Ordering::SeqCst))
            },
        )
        .expect("configuration should be valid"),
    );
    let worker_generator = Arc::clone(&generator);
    let worker = thread::spawn(move || worker_generator.next_id());

    let deadline = Instant::now() + Duration::from_secs(2);
    while call_count.load(Ordering::SeqCst) < 2
        && !worker.is_finished()
        && Instant::now() < deadline
    {
        thread::yield_now();
    }
    assert!(
        call_count.load(Ordering::SeqCst) >= 2 || worker.is_finished(),
        "the worker should either enter the wait loop or finish"
    );
    assert!(
        !worker.is_finished(),
        "the first call should wait for the next time slice"
    );
    current_millis.store(11, Ordering::SeqCst);

    let id = worker
        .join()
        .expect("worker should finish")
        .expect("id should generate after the clock advances");
    assert_eq!(QubitSnowflakeLayout::decode(id).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(id).sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_concurrent_overflow_is_unique() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let clock = Arc::new(CoordinatedClock::new(epoch));
    let generator_clock = Arc::clone(&clock);
    let generator = Arc::new(
        QubitSnowflakeGenerator::with_clock(
            IdMode::Sequential,
            TimestampPrecision::Millisecond,
            3,
            epoch,
            DEFAULT_MAX_SKEW_MILLIS,
            move || generator_clock.now(),
        )
        .expect("configuration should be valid"),
    );

    loop {
        let id = generator.next_id().expect("id should generate");
        if QubitSnowflakeLayout::decode(id).timestamp() == 10
            && QubitSnowflakeLayout::decode(id).sequence() == 4_095
        {
            break;
        }
    }

    clock.start_race();
    let mut workers = Vec::new();
    for _ in 0..2 {
        let generator = Arc::clone(&generator);
        workers.push(thread::spawn(move || generator.next_id()));
    }
    clock.wait_for_workers(2);
    clock.advance_to(11);

    let mut ids = Vec::new();
    for worker in workers {
        ids.push(
            worker
                .join()
                .expect("worker should finish")
                .expect("id should generate after the clock advances"),
        );
    }
    let timestamps = ids
        .iter()
        .map(|id| QubitSnowflakeLayout::decode(*id).timestamp())
        .collect::<HashSet<_>>();
    let sequences = ids
        .iter()
        .map(|id| QubitSnowflakeLayout::decode(*id).sequence())
        .collect::<HashSet<_>>();

    assert_eq!(timestamps, HashSet::from([11]));
    assert_eq!(sequences, HashSet::from([0, 1]));
}

#[test]
fn test_qubit_snowflake_generator_restart_skips_previous_time_slice() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let clock = Arc::new(CoordinatedClock::new(epoch));
    let first_clock = Arc::clone(&clock);
    let first_generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || first_clock.now(),
    )
    .expect("configuration should be valid");
    let first = first_generator.next_id().expect("first id should generate");
    assert_eq!(QubitSnowflakeLayout::decode(first).timestamp(), 10);
    drop(first_generator);

    let second_clock = Arc::clone(&clock);
    let second_generator = Arc::new(
        QubitSnowflakeGenerator::with_clock(
            IdMode::Sequential,
            TimestampPrecision::Millisecond,
            3,
            epoch,
            DEFAULT_MAX_SKEW_MILLIS,
            move || second_clock.now(),
        )
        .expect("configuration should be valid"),
    );
    let calls_before_restart = clock.call_count.load(Ordering::SeqCst);
    let worker_generator = Arc::clone(&second_generator);
    let worker = thread::spawn(move || worker_generator.next_id());
    let deadline = Instant::now() + Duration::from_secs(2);
    while clock.call_count.load(Ordering::SeqCst) < calls_before_restart + 2
        && !worker.is_finished()
        && Instant::now() < deadline
    {
        thread::yield_now();
    }
    assert!(!worker.is_finished());
    clock.advance_to(11);

    let second = worker
        .join()
        .expect("worker should finish")
        .expect("replacement generator should generate");
    assert_eq!(QubitSnowflakeLayout::decode(second).timestamp(), 11);
    assert_ne!(first, second);
}

#[test]
fn test_qubit_snowflake_generator_recovers_after_clock_panics() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || match clock_calls.fetch_add(1, Ordering::SeqCst) {
            0 => panic!("test clock panic"),
            1 => epoch + Duration::from_millis(10),
            _ => epoch + Duration::from_millis(11),
        },
    )
    .expect("configuration should be valid");

    let panic = catch_unwind(AssertUnwindSafe(|| generator.next_id()));
    assert!(panic.is_err(), "the first clock call should panic");

    let id = generator
        .next_id()
        .expect("generator should recover after the clock panic");
    assert_eq!(QubitSnowflakeLayout::decode(id).timestamp(), 11);
    assert_eq!(QubitSnowflakeLayout::decode(id).sequence(), 0);
}

#[test]
fn test_qubit_snowflake_generator_reports_timestamp_overflow_from_time() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch,
    )
    .expect("configuration should be valid");
    let timestamp = generator.layout().max_timestamp() + 1;

    assert_eq!(
        generator.generate_at(epoch + Duration::from_millis(timestamp), 0),
        Err(IdError::TimestampOverflow {
            timestamp,
            max: generator.layout().max_timestamp(),
        })
    );
}

#[test]
fn test_qubit_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = QubitSnowflakeGenerator::with_clock(
        IdMode::Sequential,
        TimestampPrecision::Millisecond,
        3,
        epoch,
        DEFAULT_MAX_SKEW_MILLIS,
        move || epoch,
    )
    .expect("configuration should be valid");

    assert_eq!(
        generator.generate_at(epoch - Duration::from_millis(1), 0),
        Err(IdError::TimeBeforeEpoch)
    );
}

#[test]
fn test_qubit_snowflake_generator_rejects_invalid_host_from_clock_constructor()
{
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);

    assert!(matches!(
        QubitSnowflakeGenerator::with_clock(
            IdMode::Sequential,
            TimestampPrecision::Millisecond,
            512,
            epoch,
            DEFAULT_MAX_SKEW_MILLIS,
            move || epoch,
        ),
        Err(IdError::HostOutOfRange {
            host: 512,
            max: 511
        })
    ));
}

#[test]
fn test_qubit_snowflake_generator_is_thread_safe() {
    let generator = Arc::new(
        QubitSnowflakeGenerator::new(11).expect("host should be valid"),
    );
    let mut handles = Vec::new();

    for _ in 0..4 {
        let generator = Arc::clone(&generator);
        handles.push(thread::spawn(move || {
            let mut ids = Vec::new();
            for _ in 0..128 {
                ids.push(generator.next_id().expect("id should generate"));
            }
            ids
        }));
    }

    let mut ids = HashSet::new();
    for handle in handles {
        for id in handle.join().expect("thread should finish") {
            assert!(ids.insert(id), "duplicate id generated: {id}");
        }
    }
}
