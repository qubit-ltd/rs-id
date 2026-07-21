// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous contract for ID generators.

use std::sync::Arc;

use crate::IdError;

/// Generates identifiers synchronously.
///
/// The output and error types are trait parameters so applications can inject
/// a generator through an object-safe boundary such as
/// `Arc<dyn IdGenerator<u64>>`. The error type defaults to [`IdError`].
/// Implementations that mutate allocation state must synchronize that state
/// internally because generation uses a shared reference and may be called
/// concurrently.
pub trait IdGenerator<T: Send + 'static, E = IdError>: Send + Sync {
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
    /// Returns `E` when the implementation cannot generate an identifier.
    fn generate(&self) -> Result<T, E>;
}

impl<T, E, G> IdGenerator<T, E> for Arc<G>
where
    T: Send + 'static,
    G: IdGenerator<T, E> + ?Sized,
{
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
    fn generate(&self) -> Result<T, E> {
        self.as_ref().generate()
    }
}
