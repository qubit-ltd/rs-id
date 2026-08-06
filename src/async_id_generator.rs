// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Asynchronous contract for ID generators.

use std::sync::Arc;

use crate::IdGenerationFuture;

/// Generates identifiers asynchronously.
///
/// The output and error types are associated with each generator so
/// applications can inject a generator through an object-safe boundary such as
/// `Arc<dyn AsyncIdGenerator<Output = String, Error = MyError>>`.
/// Implementations that mutate allocation state must synchronize that state
/// internally because generation uses a shared reference and may be called
/// concurrently.
pub trait AsyncIdGenerator: Send + Sync {
    /// Value produced by this generator.
    type Output: Send + 'static;
    /// Error returned by this generator.
    type Error;

    /// Generates the next identifier asynchronously.
    ///
    /// Implementations should yield while waiting for time or other external
    /// progress and must not block an asynchronous executor thread.
    ///
    /// # Returns
    ///
    /// A future that resolves to the next generated identifier.
    ///
    /// # Errors
    ///
    /// The returned future resolves to [`Self::Error`] when the implementation
    /// cannot generate an identifier.
    fn generate_async(
        &self,
    ) -> IdGenerationFuture<'_, Self::Output, Self::Error>;
}

impl<G> AsyncIdGenerator for Arc<G>
where
    G: AsyncIdGenerator + ?Sized,
{
    type Output = G::Output;
    type Error = G::Error;

    /// Delegates asynchronous identifier generation to the shared generator.
    ///
    /// # Returns
    ///
    /// A future that resolves to the identifier returned by the wrapped
    /// generator.
    ///
    /// # Errors
    ///
    /// The returned future resolves to any error produced by the wrapped
    /// generator.
    #[inline(always)]
    fn generate_async(
        &self,
    ) -> IdGenerationFuture<'_, Self::Output, Self::Error> {
        self.as_ref().generate_async()
    }
}
