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

use self::id_generator_support::CounterGenerator;

#[allow(dead_code)]
struct LocalOutput(Rc<()>);

struct LocalOutputGenerator;

impl IdGenerator for LocalOutputGenerator {
    type Output = LocalOutput;
    type Error = std::convert::Infallible;
}

#[test]
fn test_id_generator_allows_non_send_synchronous_output_type() {
    fn require_id_generator<G>(_: &G)
    where
        G: IdGenerator<Output = LocalOutput, Error = std::convert::Infallible>,
    {
    }

    let generator = LocalOutputGenerator;
    require_id_generator(&generator);
}

#[test]
fn test_id_generator_is_object_safe_for_one_output_type() {
    let generator: Arc<
        dyn IdGenerator<Output = u64, Error = qubit_id::IdGenerationError>,
    > = Arc::new(CounterGenerator::default());

    fn require_id_generator(
        _: &dyn IdGenerator<Output = u64, Error = qubit_id::IdGenerationError>,
    ) {
    }

    require_id_generator(generator.as_ref());

    let shared = Arc::new(CounterGenerator::default());
    require_id_generator(&shared);
}
