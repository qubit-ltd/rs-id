// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Decimal-string adapter for Snowflake-family generators.

use crate::{
    AsyncIdGenerator,
    IdError,
    IdGenerationFuture,
    IdGenerator,
};

/// Adapts a numeric Snowflake generator to decimal [`String`] output.
///
/// The adapter supports the synchronous and asynchronous generator contracts
/// independently according to the capabilities implemented by `G`.
#[derive(Debug)]
#[must_use]
pub struct SnowflakeStringGenerator<G> {
    /// Numeric generator wrapped by this adapter.
    inner: G,
}

impl<G> SnowflakeStringGenerator<G> {
    /// Creates a decimal-string adapter for `inner`.
    ///
    /// # Arguments
    ///
    /// * `inner` - Numeric Snowflake generator to wrap.
    ///
    /// # Returns
    ///
    /// An adapter that preserves the generation mode of `inner`.
    #[inline(always)]
    pub const fn new(inner: G) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the wrapped numeric generator.
    #[must_use]
    #[inline(always)]
    pub const fn inner(&self) -> &G {
        &self.inner
    }

    /// Consumes the adapter and returns the wrapped numeric generator.
    #[must_use]
    #[inline(always)]
    pub fn into_inner(self) -> G {
        self.inner
    }
}

impl<G> IdGenerator<String> for SnowflakeStringGenerator<G>
where
    G: IdGenerator<u64>,
{
    /// Generates a Snowflake ID as unsigned decimal text.
    ///
    /// # Returns
    ///
    /// The next numeric ID formatted as a decimal string.
    ///
    /// # Errors
    ///
    /// Returns the [`IdError`] produced by the wrapped generator.
    #[inline(always)]
    fn generate(&self) -> Result<String, IdError> {
        self.inner.generate().map(|id| id.to_string())
    }
}

impl<G> AsyncIdGenerator<String> for SnowflakeStringGenerator<G>
where
    G: AsyncIdGenerator<u64>,
{
    /// Generates a Snowflake ID as unsigned decimal text asynchronously.
    ///
    /// # Returns
    ///
    /// A future that awaits the wrapped generator and formats its output.
    ///
    /// # Errors
    ///
    /// The future resolves to the [`IdError`] produced by the wrapped
    /// generator.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, String> {
        Box::pin(async move {
            self.inner.generate_async().await.map(|id| id.to_string())
        })
    }
}
