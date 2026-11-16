// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the blocking ID-generator capability.

mod id_generator_support;

use std::io::Error;
use std::sync::Arc;
use std::thread;

use qubit_id::IdGenerator;

use self::id_generator_support::counter_generator_tests::CounterGenerator;
use self::id_generator_support::counter_generator_tests::IoCounterGenerator;

#[test]
fn test_blocking_id_generator_supports_custom_error_type() {
    let generator = Arc::new(IoCounterGenerator::default());

    assert_eq!(
        <Arc<IoCounterGenerator> as IdGenerator<u64, Error>>::generate(&generator).expect("generation should succeed"),
        1
    );
}

#[test]
fn test_blocking_id_generator_supports_concurrent_shared_access() {
    let generator: Arc<dyn IdGenerator<u64>> = Arc::new(CounterGenerator::default());
    let first_generator = Arc::clone(&generator);
    let second_generator = Arc::clone(&generator);
    let first = thread::spawn(move || first_generator.generate().expect("generation should succeed"));
    let second = thread::spawn(move || second_generator.generate().expect("generation should succeed"));
    let mut generated = [
        first.join().expect("first thread should finish"),
        second.join().expect("second thread should finish"),
    ];
    generated.sort_unstable();

    assert_eq!(generated, [1, 2]);
}
