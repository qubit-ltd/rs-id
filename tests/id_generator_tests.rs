// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the `IdGenerator` trait contract.

mod id_generator_support;

use std::io::ErrorKind;

use qubit_id::{
    GenerationOutcome,
    IdGenerator,
};

use self::id_generator_support::{
    FailingGenerator,
    FixedGenerator,
};

#[test]
fn test_id_generator_formats_id_without_display() {
    let generator = FixedGenerator::new(42);

    assert_eq!(
        generator
            .next_string()
            .expect("fixed generation should succeed"),
        "opaque:42"
    );
    assert_eq!(
        generator
            .try_next_string()
            .expect("fixed generation should succeed"),
        GenerationOutcome::Generated("opaque:42".to_owned())
    );
}

#[test]
fn test_id_generator_try_next_string_propagates_error() {
    let error = FailingGenerator
        .try_next_string()
        .expect_err("failing generation should return its error");

    assert_eq!(error.kind(), ErrorKind::Other);
}
