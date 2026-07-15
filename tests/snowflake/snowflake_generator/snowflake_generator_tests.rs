// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the classic Snowflake generator.

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
    SystemTime,
    UNIX_EPOCH,
};

use qubit_id::{
    IdError,
    IdGenerator,
    SnowflakeGenerator,
};

/// Clock that keeps overflow waiters in one millisecond until released.
struct CoordinatedClock {
    epoch: SystemTime,
    call_count: AtomicU64,
    current_millis: AtomicU64,
    race_started: AtomicBool,
    workers: Mutex<HashSet<thread::ThreadId>>,
    workers_changed: Condvar,
}

impl CoordinatedClock {
    /// Creates a clock that moves from millisecond 9 to millisecond 10.
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
    fn advance_to(&self, timestamp: u64) {
        self.current_millis.store(timestamp, Ordering::SeqCst);
    }
}

#[test]
fn test_snowflake_generator_compose_and_extract_parts() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_epoch(513, epoch)
        .expect("node id should be valid");

    let id = generator
        .compose(1_234_567, 2_117)
        .expect("parts should be valid");

    assert_eq!(generator.node_id(), 513);
    assert_eq!(generator.epoch(), epoch);
    assert_eq!(generator.extract_timestamp(id), 1_234_567);
    assert_eq!(generator.extract_node_id(id), 513);
    assert_eq!(generator.extract_sequence(id), 2_117);
}

#[test]
fn test_snowflake_generator_rejects_invalid_node_and_parts() {
    match SnowflakeGenerator::new(1_024) {
        Err(error) => assert_eq!(
            error,
            IdError::NodeOutOfRange {
                node_id: 1_024,
                max: 1_023,
            }
        ),
        Ok(_) => panic!("invalid node id should be rejected"),
    }

    let generator =
        SnowflakeGenerator::new(1).expect("node id should be valid");
    assert_eq!(
        generator.compose(generator.max_timestamp() + 1, 0),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_timestamp() + 1,
            max: generator.max_timestamp(),
        })
    );
    assert_eq!(
        generator.compose(0, generator.max_sequence() + 1),
        Err(IdError::SequenceOverflow {
            sequence: generator.max_sequence() + 1,
            max: generator.max_sequence(),
        })
    );
}

#[test]
fn test_snowflake_generator_next_string_uses_numeric_string() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        epoch + Duration::from_millis(77)
    })
    .expect("configuration should be valid");

    let id = generator.next_id().expect("id should generate");
    let next_string = generator
        .next_string()
        .expect("string id should generate after numeric id");

    assert_eq!(generator.extract_timestamp(id), 77);
    assert_eq!(next_string, (id + 1).to_string());
}

#[test]
fn test_snowflake_generator_reports_clock_backwards() {
    let current_millis = Arc::new(AtomicU64::new(10));
    let clock_millis = Arc::clone(&current_millis);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        epoch + Duration::from_millis(clock_millis.load(Ordering::SeqCst))
    })
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
fn test_snowflake_generator_reports_rollback_while_waiting() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        let timestamp = match clock_calls.fetch_add(1, Ordering::SeqCst) {
            0..=4_096 => 10,
            4_097 => 0,
            _ => 11,
        };
        epoch + Duration::from_millis(timestamp)
    })
    .expect("configuration should be valid");

    for _ in 0..=generator.max_sequence() {
        generator.next_id().expect("sequence should be available");
    }

    assert_eq!(
        generator.next_id(),
        Err(IdError::ClockMovedBackwards {
            last_timestamp: 10,
            current_timestamp: 0,
            skew_millis: 10,
            max_skew_millis: 0,
        })
    );
}

#[test]
fn test_snowflake_generator_waits_when_sequence_overflows() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        let call = clock_calls.fetch_add(1, Ordering::SeqCst);
        if call <= 4_096 {
            epoch + Duration::from_millis(10)
        } else {
            epoch + Duration::from_millis(11)
        }
    })
    .expect("configuration should be valid");

    for expected_sequence in 0..=4_095 {
        let id = generator.next_id().expect("id should generate");
        assert_eq!(generator.extract_sequence(id), expected_sequence);
    }
    let wrapped = generator
        .next_id()
        .expect("generator should wait for the next millisecond");

    assert_eq!(generator.extract_timestamp(wrapped), 11);
    assert_eq!(generator.extract_sequence(wrapped), 0);
}

#[test]
fn test_snowflake_generator_concurrent_overflow_is_unique() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let clock = Arc::new(CoordinatedClock::new(epoch));
    let generator_clock = Arc::clone(&clock);
    let generator = Arc::new(
        SnowflakeGenerator::with_clock(9, epoch, move || generator_clock.now())
            .expect("configuration should be valid"),
    );

    loop {
        let id = generator.next_id().expect("id should generate");
        if generator.extract_timestamp(id) == 10
            && generator.extract_sequence(id) == 4_095
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
        .map(|id| generator.extract_timestamp(*id))
        .collect::<HashSet<_>>();
    let sequences = ids
        .iter()
        .map(|id| generator.extract_sequence(*id))
        .collect::<HashSet<_>>();

    assert_eq!(timestamps, HashSet::from([11]));
    assert_eq!(sequences, HashSet::from([0, 1]));
}

#[test]
fn test_snowflake_generator_same_node_restart_can_repeat_id() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let first_generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        epoch + Duration::from_millis(10)
    })
    .expect("configuration should be valid");
    let first = first_generator.next_id().expect("first id should generate");
    let second_generator =
        SnowflakeGenerator::with_clock(9, epoch, move || {
            epoch + Duration::from_millis(10)
        })
        .expect("configuration should be valid");
    let second = second_generator
        .next_id()
        .expect("replacement generator should generate immediately");

    assert_eq!(first, second);
}

#[test]
fn test_snowflake_generator_first_id_uses_current_time_slice() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        let timestamp = if clock_calls.fetch_add(1, Ordering::SeqCst) == 0 {
            10
        } else {
            11
        };
        epoch + Duration::from_millis(timestamp)
    })
    .expect("configuration should be valid");

    let id = generator
        .next_id()
        .expect("first id should generate immediately");

    assert_eq!(generator.extract_timestamp(id), 10);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_snowflake_generator_recovers_after_clock_panics() {
    let call_count = Arc::new(AtomicU64::new(0));
    let clock_calls = Arc::clone(&call_count);
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        match clock_calls.fetch_add(1, Ordering::SeqCst) {
            0 => panic!("test clock panic"),
            1 => epoch + Duration::from_millis(10),
            _ => epoch + Duration::from_millis(11),
        }
    })
    .expect("configuration should be valid");

    let panic = catch_unwind(AssertUnwindSafe(|| generator.next_id()));
    assert!(panic.is_err());

    let id = generator
        .next_id()
        .expect("generator should recover after the clock panic");
    assert_eq!(generator.extract_timestamp(id), 10);
    assert_eq!(generator.extract_sequence(id), 0);
}

#[test]
fn test_snowflake_generator_reports_timestamp_overflow_from_clock() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        epoch + Duration::from_millis((1_u64 << 41) + 1)
    })
    .expect("configuration should be valid");

    assert_eq!(
        generator.next_id(),
        Err(IdError::TimestampOverflow {
            timestamp: generator.max_timestamp() + 2,
            max: generator.max_timestamp(),
        })
    );
}

#[test]
fn test_snowflake_generator_reports_time_before_epoch() {
    let epoch = UNIX_EPOCH + Duration::from_millis(1_700_000_000_000);
    let generator = SnowflakeGenerator::with_clock(9, epoch, move || {
        epoch - Duration::from_millis(1)
    })
    .expect("configuration should be valid");

    assert_eq!(generator.next_id(), Err(IdError::TimeBeforeEpoch));
}
