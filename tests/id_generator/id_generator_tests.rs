/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Tests for the `IdGenerator` trait.

use std::convert::Infallible;

use qubit_id::{
    IdError,
    IdGenerator,
};

struct FixedGenerator;

impl IdGenerator<u64> for FixedGenerator {
    type Error = Infallible;

    fn next_id(&self) -> Result<u64, Self::Error> {
        Ok(42)
    }
}

struct FailingGenerator;

impl IdGenerator<u64> for FailingGenerator {
    type Error = IdError;

    fn next_id(&self) -> Result<u64, Self::Error> {
        Err(IdError::StatePoisoned)
    }
}

#[test]
fn test_id_generator_next_string_uses_display_by_default() {
    let generator = FixedGenerator;

    let id = generator
        .next_string()
        .expect("fixed generator should not fail");

    assert_eq!(id, "42");
}

#[test]
fn test_id_generator_next_string_propagates_generation_error() {
    let generator = FailingGenerator;

    assert_eq!(generator.next_string(), Err(IdError::StatePoisoned));
}
