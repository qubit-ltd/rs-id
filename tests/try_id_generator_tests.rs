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
    TryIdGenerator,
};

struct Counter;

impl TryIdGenerator<u64> for Counter {
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<u64>, qubit_id::IdGenerationError> {
        Ok(GenerationAttempt::Generated(1))
    }
}

#[test]
fn test_try_id_generator_arc_delegates() {
    let generator: Arc<dyn TryIdGenerator<u64>> = Arc::new(Counter);
    assert!(matches!(
        generator.try_generate(),
        Ok(GenerationAttempt::Generated(1))
    ));
}
