// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the `IdGenerator` trait contract.

mod id_generator_support;

use std::sync::Arc;

use qubit_id::IdGenerator;

use self::id_generator_support::CounterGenerator;

#[test]
fn test_id_generator_is_object_safe_for_one_output_type() {
    let generator: Arc<dyn IdGenerator<u64>> =
        Arc::new(CounterGenerator::default());

    assert_eq!(generator.generate().expect("generation should succeed"), 1);
    assert_eq!(generator.generate().expect("generation should succeed"), 2);
}

#[test]
fn test_id_generator_supports_custom_error_type() {
    let generator: Arc<dyn IdGenerator<u64, std::io::Error>> =
        Arc::new(CounterGenerator::default());

    assert_eq!(generator.generate().expect("generation should succeed"), 1);
}

#[test]
fn test_id_generator_supports_concurrent_shared_access() {
    let generator: Arc<dyn IdGenerator<u64>> =
        Arc::new(CounterGenerator::default());
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
