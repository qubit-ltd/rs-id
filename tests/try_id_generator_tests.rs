// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Tests for the non-blocking generator contract.

use std::sync::Arc;

use qubit_id::{
    GenerationAttempt,
    IdGenerator,
    TryIdGenerator,
};

struct Counter;

impl TryIdGenerator for Counter {
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Self::Output>, Self::Error> {
        Ok(GenerationAttempt::Generated(1))
    }
}

impl IdGenerator for Counter {
    type Output = u64;
    type Error = qubit_id::IdGenerationError;
}

#[test]
fn test_try_id_generator_arc_delegates() {
    let generator: Arc<
        dyn TryIdGenerator<Output = u64, Error = qubit_id::IdGenerationError>,
    > = Arc::new(Counter);
    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(1))
    ));
}
