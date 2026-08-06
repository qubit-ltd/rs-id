// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous contract for ID generators.

use std::sync::Arc;

/// Generates identifiers synchronously.
///
/// The output and error types are associated with each generator so
/// applications can inject a generator through an object-safe boundary such as
/// `Arc<dyn IdGenerator<Output = u64, Error = MyError>>`.
/// Implementations that mutate allocation state must synchronize that state
/// internally because generation uses a shared reference and may be called
/// concurrently.
pub trait IdGenerator: Send + Sync {
    /// Value produced by this generator.
    type Output: Send + 'static;
    /// Error returned by this generator.
    type Error;

    /// Generates the next identifier.
    ///
    /// This method may block when an implementation must wait for time to
    /// advance or another retryable condition to clear.
    ///
    /// # Returns
    ///
    /// The next generated identifier.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the implementation cannot generate an
    /// identifier.
    fn generate(&self) -> Result<Self::Output, Self::Error>;
}

impl<G> IdGenerator for Arc<G>
where
    G: IdGenerator + ?Sized,
{
    type Output = G::Output;
    type Error = G::Error;

    /// Delegates identifier generation to the shared generator.
    ///
    /// # Returns
    ///
    /// The next identifier returned by the wrapped generator.
    ///
    /// # Errors
    ///
    /// Returns any error produced by the wrapped generator.
    #[inline(always)]
    fn generate(&self) -> Result<Self::Output, Self::Error> {
        self.as_ref().generate()
    }
}
