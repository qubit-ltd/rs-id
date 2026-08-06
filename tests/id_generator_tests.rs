// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the `IdGenerator` trait contract.

mod id_generator_support;

use std::rc::Rc;
use std::sync::Arc;

use qubit_id::IdGenerator;

use self::id_generator_support::{
    CounterGenerator,
    IoCounterGenerator,
};

struct LocalOutput(Rc<()>);

struct LocalOutputGenerator;

impl IdGenerator for LocalOutputGenerator {
    type Output = LocalOutput;
    type Error = std::convert::Infallible;

    /// Generates a synchronous output that is intentionally not `Send`.
    fn generate(&self) -> Result<Self::Output, Self::Error> {
        Ok(LocalOutput(Rc::new(())))
    }
}

#[test]
fn test_id_generator_allows_non_send_synchronous_output() {
    let output = LocalOutputGenerator
        .generate()
        .expect("local output generation should succeed");

    assert_eq!(Rc::strong_count(&output.0), 1);
}

#[test]
fn test_id_generator_is_object_safe_for_one_output_type() {
    let generator: Arc<
        dyn IdGenerator<Output = u64, Error = qubit_id::IdGenerationError>,
    > = Arc::new(CounterGenerator::default());

    assert_eq!(generator.generate().expect("generation should succeed"), 1);
    assert_eq!(generator.generate().expect("generation should succeed"), 2);
}

#[test]
fn test_id_generator_supports_custom_error_type() {
    let generator: Arc<dyn IdGenerator<Output = u64, Error = std::io::Error>> =
        Arc::new(IoCounterGenerator::default());

    assert_eq!(generator.generate().expect("generation should succeed"), 1);
}

#[test]
fn test_id_generator_supports_concurrent_shared_access() {
    let generator: Arc<
        dyn IdGenerator<Output = u64, Error = qubit_id::IdGenerationError>,
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
