// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a thread-safe counter generator fixture.

use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

use qubit_id::{
    AsyncIdGenerator,
    IdGenerationFuture,
    IdGenerator,
};

/// Generator that increments an atomic counter for each request.
#[derive(Debug, Default)]
pub(crate) struct CounterGenerator {
    /// Last numeric value returned by the fixture.
    value: AtomicU64,
}

impl IdGenerator<u64> for CounterGenerator {
    /// Increments and returns the fixture counter.
    ///
    /// # Returns
    ///
    /// The next positive counter value.
    ///
    /// # Errors
    ///
    /// This fixture does not return an error.
    #[inline(always)]
    fn generate(&self) -> Result<u64, qubit_id::IdGenerationError> {
        Ok(self.value.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

impl AsyncIdGenerator<u64> for CounterGenerator {
    /// Asynchronously increments and returns the fixture counter.
    ///
    /// # Returns
    ///
    /// An immediately ready future containing the next counter value.
    #[inline(always)]
    fn generate_async(
        &self,
    ) -> IdGenerationFuture<'_, u64, qubit_id::IdGenerationError> {
        Box::pin(async move { <Self as IdGenerator<u64>>::generate(self) })
    }
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct IoCounterGenerator {
    /// Last numeric value returned by the fixture.
    value: AtomicU64,
}

impl IdGenerator<u64, std::io::Error> for IoCounterGenerator {
    /// Increments and returns the fixture counter with a custom error type.
    ///
    /// # Returns
    ///
    /// The next positive counter value.
    ///
    /// # Errors
    ///
    /// This fixture does not return an error.
    #[inline(always)]
    fn generate(&self) -> Result<u64, std::io::Error> {
        Ok(self.value.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

impl AsyncIdGenerator<u64, std::io::Error> for IoCounterGenerator {
    /// Asynchronously increments and returns the fixture counter with a custom
    /// error type.
    ///
    /// # Returns
    ///
    /// An immediately ready future containing the next counter value.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, u64, std::io::Error> {
        Box::pin(async move {
            <Self as IdGenerator<u64, std::io::Error>>::generate(self)
        })
    }
}
