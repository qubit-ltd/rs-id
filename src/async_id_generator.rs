// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Asynchronous contract for ID generators.

use std::sync::Arc;

use crate::{
    Id,
    IdGenerationError,
    IdGenerationFuture,
    IdGenerator,
};

/// Generates identifiers asynchronously.
///
/// This capability inherits its output and error types from [`IdGenerator`],
/// so applications can inject a generator through an object-safe boundary such
/// as `Arc<dyn AsyncIdGenerator<String, MyError>>`.
/// Implementations that mutate allocation state must synchronize that state
/// internally because generation uses a shared reference and may be called
/// concurrently.
pub trait AsyncIdGenerator<Output = Id, Error = IdGenerationError>:
    IdGenerator<Output, Error>
where
    Output: Send + 'static,
{
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
    /// The returned future resolves to `Error` when the implementation cannot
    /// generate an identifier.
    fn generate_async(&self) -> IdGenerationFuture<'_, Output, Error>;
}

impl<Output, Error, G> AsyncIdGenerator<Output, Error> for Arc<G>
where
    G: AsyncIdGenerator<Output, Error> + ?Sized,
    Output: Send + 'static,
{
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
    fn generate_async(&self) -> IdGenerationFuture<'_, Output, Error> {
        self.as_ref().generate_async()
    }
}
