// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the blocking ID-generator capability.

mod id_generator_support;

use std::sync::Arc;

use qubit_id::BlockingIdGenerator;

use self::id_generator_support::{
    CounterGenerator,
    IoCounterGenerator,
};

#[test]
fn test_blocking_id_generator_supports_custom_error_type() {
    let generator = Arc::new(IoCounterGenerator::default());

    assert_eq!(
        <Arc<IoCounterGenerator> as BlockingIdGenerator>::generate(&generator)
            .expect("generation should succeed"),
        1
    );
}

#[test]
fn test_blocking_id_generator_supports_concurrent_shared_access() {
    let generator: Arc<
        dyn BlockingIdGenerator<
                Output = u64,
                Error = qubit_id::IdGenerationError,
            >,
    > = Arc::new(CounterGenerator::default());
    let first_generator = Arc::clone(&generator);
    let second_generator = Arc::clone(&generator);
    let first = std::thread::spawn(move || {
        first_generator
            .generate()
            .expect("generation should succeed")
    });
    let second = std::thread::spawn(move || {
        second_generator
            .generate()
            .expect("generation should succeed")
    });
    let mut generated = [
        first.join().expect("first thread should finish"),
        second.join().expect("second thread should finish"),
    ];
    generated.sort_unstable();

    assert_eq!(generated, [1, 2]);
}
