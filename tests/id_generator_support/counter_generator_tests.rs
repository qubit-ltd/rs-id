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
    BlockingIdGenerator,
    IdGenerationFuture,
    IdGenerator,
};

/// Generator that increments an atomic counter for each request.
#[derive(Debug, Default)]
pub(crate) struct CounterGenerator {
    /// Last numeric value returned by the fixture.
    value: AtomicU64,
}

impl IdGenerator for CounterGenerator {
    type Output = u64;
    type Error = qubit_id::IdGenerationError;
}

impl BlockingIdGenerator for CounterGenerator {
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
    fn generate(&self) -> Result<Self::Output, Self::Error> {
        Ok(self.value.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

impl AsyncIdGenerator for CounterGenerator {
    /// Asynchronously increments and returns the fixture counter.
    ///
    /// # Returns
    ///
    /// An immediately ready future containing the next counter value.
    #[inline(always)]
    fn generate_async(
        &self,
    ) -> IdGenerationFuture<'_, Self::Output, Self::Error> {
        Box::pin(async move { <Self as BlockingIdGenerator>::generate(self) })
    }
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct IoCounterGenerator {
    /// Last numeric value returned by the fixture.
    value: AtomicU64,
}

impl IdGenerator for IoCounterGenerator {
    type Output = u64;
    type Error = std::io::Error;
}

impl BlockingIdGenerator for IoCounterGenerator {
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
    fn generate(&self) -> Result<Self::Output, Self::Error> {
        Ok(self.value.fetch_add(1, Ordering::Relaxed) + 1)
    }
}

impl AsyncIdGenerator for IoCounterGenerator {
    /// Asynchronously increments and returns the fixture counter with a custom
    /// error type.
    ///
    /// # Returns
    ///
    /// An immediately ready future containing the next counter value.
    #[inline(always)]
    fn generate_async(
        &self,
    ) -> IdGenerationFuture<'_, Self::Output, Self::Error> {
        Box::pin(async move { <Self as BlockingIdGenerator>::generate(self) })
    }
}
