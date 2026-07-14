// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the Sonyflake-style generator.

use std::collections::HashSet;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::atomic::{
    AtomicBool,
    AtomicI64,
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
    IdError,
    IdGenerator,
    SonyflakeGenerator,
};

/// Clock that keeps overflow waiters in one time unit until released.
struct CoordinatedClock {
    epoch: SystemTime,
    call_count: AtomicU64,
    current_millis: AtomicU64,
    race_started: AtomicBool,
    workers: Mutex<HashSet<thread::ThreadId>>,
    workers_changed: Condvar,
}

impl CoordinatedClock {
    /// Creates a clock whose startup-fence read is 9 ms and later reads are 10
    /// ms.
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

    /// Returns test time and records workers participating in the race.
    fn now(&self) -> SystemTime {
        let call = self.call_count.fetch_add(1, Ordering::SeqCst);
        if call == 1 {
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

    /// Starts recording overflow workers.
    fn start_race(&self) {
        self.race_started.store(true, Ordering::SeqCst);
    }

    /// Waits until every overflow worker has read the clock.
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
        assert_eq!(workers.len(), expected);
    }

    /// Advances the test clock.
    fn advance_to(&self, elapsed_millis: u64) {
        self.current_millis.store(elapsed_millis, Ordering::SeqCst);
    }
}

#[test]
fn test_sonyflake_generator_default_layout_matches_sonyflake() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        0x1234,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        move || epoch + Duration::from_millis(120),
    )
    .expect("configuration should be valid");

    let id = generator
        .compose(12, 7, 0x1234)
        .expect("parts should be valid");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
    assert_eq!(id, (12_u64 << 24) | (7_u64 << 16) | 0x1234);
    assert_eq!(generator.extract_elapsed_time(id), 12);
    assert_eq!(generator.extract_sequence(id), 7);
    assert_eq!(generator.extract_machine_id(id), 0x1234);
}

#[test]
fn test_sonyflake_generator_new_uses_default_layout() {
    let generator =
        SonyflakeGenerator::new(1).expect("default machine id should be valid");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
}

#[test]
fn test_sonyflake_generator_zero_bits_select_defaults() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        1,
        0,
        0,
        Duration::from_millis(10),
        epoch,
        move || epoch,
    )
    .expect("zero bit lengths should select defaults");

    assert_eq!(generator.bits_time(), 39);
    assert_eq!(generator.bits_sequence(), 8);
    assert_eq!(generator.bits_machine(), 16);
}

#[test]
fn test_sonyflake_generator_next_id_waits_for_physical_next_time_unit() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let current_millis = Arc::new(AtomicU64::new(5));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        1,
        1,
        1,
        Duration::from_millis(1),
        epoch,
        move || {
            let millis = match clock_calls.fetch_add(1, Ordering::SeqCst) {
                1 => 4,
                _ => clock_millis.load(Ordering::SeqCst),
            };
            epoch + Duration::from_millis(millis)
        },
    )
    .expect("configuration should be valid");

    let first = generator.next_id().expect("first id should generate");
    let second = generator.next_id().expect("second id should generate");
    let generator = Arc::new(generator);
    let third_generator = Arc::clone(&generator);
    let third = thread::spawn(move || third_generator.next_id());

    let deadline = Instant::now() + Duration::from_secs(2);
    while call_count.load(Ordering::SeqCst) < 7
        && !third.is_finished()
        && Instant::now() < deadline
    {
        thread::yield_now();
    }
    assert!(!third.is_finished());
    current_millis.store(6, Ordering::SeqCst);
    let third = third
        .join()
        .expect("worker should finish")
        .expect("third id should generate");

    assert_eq!(generator.extract_elapsed_time(first), 5);
    assert_eq!(generator.extract_sequence(first), 0);
    assert_eq!(generator.extract_elapsed_time(second), 5);
    assert_eq!(generator.extract_sequence(second), 1);
    assert_eq!(generator.extract_elapsed_time(third), 6);
    assert_eq!(generator.extract_sequence(third), 0);
}

#[test]
fn test_sonyflake_generator_concurrent_overflow_is_unique() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let clock = Arc::new(CoordinatedClock::new(epoch));
    let generator_clock = Arc::clone(&clock);
    let generator = Arc::new(
        SonyflakeGenerator::with_clock(
            1,
            1,
            1,
            Duration::from_millis(1),
            epoch,
            move || generator_clock.now(),
        )
        .expect("configuration should be valid"),
    );

    loop {
        let id = generator.next_id().expect("id should generate");
        if generator.extract_elapsed_time(id) == 10
            && generator.extract_sequence(id) == 1
        {
            break;
        }
    }

    clock.start_race();
    let workers = (0..2)
        .map(|_| {
            let generator = Arc::clone(&generator);
            thread::spawn(move || generator.next_id())
        })
        .collect::<Vec<_>>();
    clock.wait_for_workers(2);
    clock.advance_to(11);

    let ids = workers
        .into_iter()
        .map(|worker| {
            worker
                .join()
                .expect("worker should finish")
                .expect("id should generate after the clock advances")
        })
        .collect::<Vec<_>>();
    let timestamps = ids
        .iter()
        .map(|id| generator.extract_elapsed_time(*id))
        .collect::<HashSet<_>>();
    let sequences = ids
        .iter()
        .map(|id| generator.extract_sequence(*id))
        .collect::<HashSet<_>>();

    assert_eq!(timestamps, HashSet::from([11]));
    assert_eq!(sequences, HashSet::from([0, 1]));
}

#[test]
fn test_sonyflake_generator_restart_skips_previous_time_unit() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let clock = Arc::new(CoordinatedClock::new(epoch));
    let first_clock = Arc::clone(&clock);
    let first_generator = SonyflakeGenerator::with_clock(
        1,
        1,
        1,
        Duration::from_millis(1),
        epoch,
        move || first_clock.now(),
    )
    .expect("configuration should be valid");
    let first = first_generator.next_id().expect("first id should generate");
    assert_eq!(first_generator.extract_elapsed_time(first), 10);
    drop(first_generator);

    let second_clock = Arc::clone(&clock);
    let second_generator = Arc::new(
        SonyflakeGenerator::with_clock(
            1,
            1,
            1,
            Duration::from_millis(1),
            epoch,
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
    assert_eq!(second_generator.extract_elapsed_time(second), 11);
    assert_ne!(first, second);
}

#[test]
fn test_sonyflake_generator_rejects_invalid_settings_and_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);

    match SonyflakeGenerator::with_epoch(65_536, epoch) {
        Err(error) => assert_eq!(
            error,
            IdError::MachineIdOutOfRange {
                machine_id: 65_536,
                max: 65_535,
            }
        ),
        Ok(_) => panic!("invalid machine id should be rejected"),
    }
    assert!(matches!(
        SonyflakeGenerator::with_options(
            1,
            31,
            1,
            Duration::from_millis(10),
            epoch
        ),
        Err(IdError::InvalidBitLength {
            name: "sequence",
            ..
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::with_options(
            1,
            30,
            2,
            Duration::from_millis(10),
            epoch
        ),
        Err(IdError::InvalidBitLength {
            name: "time",
            bits: 31,
            ..
        })
    ));
    assert!(matches!(
        SonyflakeGenerator::with_clock(
            1,
            8,
            16,
            Duration::from_millis(10),
            epoch + Duration::from_millis(1),
            move || epoch,
        ),
        Err(IdError::StartTimeAhead)
    ));
    match SonyflakeGenerator::with_options(
        1,
        8,
        16,
        Duration::from_nanos(1),
        epoch,
    ) {
        Err(error) => assert_eq!(
            error,
            IdError::InvalidTimeUnit {
                nanos: 1,
                min_nanos: 1_000_000,
            }
        ),
        Ok(_) => panic!("invalid time unit should be rejected"),
    }

    let generator = SonyflakeGenerator::with_epoch(1, epoch)
        .expect("machine id should be valid");
    assert_eq!(
        generator.compose(generator.max_elapsed_time() + 1, 0, 1),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_elapsed_time() + 1,
            max: generator.max_elapsed_time(),
        })
    );
    assert_eq!(
        generator.compose(0, generator.max_sequence() + 1, 1),
        Err(IdError::SequenceOverflow {
            sequence: generator.max_sequence() + 1,
            max: generator.max_sequence(),
        })
    );
    assert_eq!(
        generator.compose(0, 0, generator.max_machine_id() + 1),
        Err(IdError::MachineIdOutOfRange {
            machine_id: generator.max_machine_id() + 1,
            max: generator.max_machine_id(),
        })
    );
}

#[test]
fn test_sonyflake_generator_string_output_is_numeric() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        7,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        move || {
            let millis = if clock_calls.fetch_add(1, Ordering::SeqCst) == 1 {
                9
            } else {
                10
            };
            epoch + Duration::from_millis(millis)
        },
    )
    .expect("configuration should be valid");

    let id = generator.next_id().expect("id should generate");

    assert_eq!(
        generator
            .next_string()
            .expect("string id should generate after numeric id"),
        (id + (1_u64 << 16)).to_string()
    );
}

#[test]
fn test_sonyflake_generator_reports_clock_backwards() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let current_millis = Arc::new(AtomicU64::new(10));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        7,
        8,
        16,
        Duration::from_millis(1),
        epoch,
        move || {
            let millis = if clock_calls.fetch_add(1, Ordering::SeqCst) == 1 {
                9
            } else {
                clock_millis.load(Ordering::SeqCst)
            };
            epoch + Duration::from_millis(millis)
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
fn test_sonyflake_generator_recovers_after_clock_panics() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        7,
        8,
        16,
        Duration::from_millis(1),
        epoch,
        move || match clock_calls.fetch_add(1, Ordering::SeqCst) {
            0 => epoch + Duration::from_millis(9),
            1 => panic!("test clock panic"),
            2 => epoch + Duration::from_millis(10),
            _ => epoch + Duration::from_millis(11),
        },
    )
    .expect("configuration should be valid");

    let panic = catch_unwind(AssertUnwindSafe(|| generator.next_id()));
    assert!(panic.is_err());

    let id = generator
        .next_id()
        .expect("generator should recover after the clock panic");
    assert_eq!(generator.extract_elapsed_time(id), 11);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_sonyflake_generator_reports_time_before_epoch_after_construction() {
    let offset = Arc::new(AtomicI64::new(0));
    let clock_offset = Arc::clone(&offset);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        7,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        move || {
            let millis = clock_offset.load(Ordering::SeqCst);
            if millis >= 0 {
                epoch + Duration::from_millis(millis as u64)
            } else {
                epoch - Duration::from_millis(millis.unsigned_abs())
            }
        },
    )
    .expect("construction clock should be at epoch");

    offset.store(-1, Ordering::SeqCst);

    assert_eq!(generator.next_id(), Err(IdError::TimeBeforeEpoch));
}

#[test]
fn test_sonyflake_generator_reports_timestamp_overflow_from_clock() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_735_689_600_000);
    let generator = SonyflakeGenerator::with_clock(
        7,
        8,
        16,
        Duration::from_millis(10),
        epoch,
        move || epoch + Duration::from_millis((1_u64 << 39) * 10),
    )
    .expect("configuration should be valid");

    assert_eq!(
        generator.next_id(),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_elapsed_time() + 1,
            max: generator.max_elapsed_time(),
        })
    );
}
